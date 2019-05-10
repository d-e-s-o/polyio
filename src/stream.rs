// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;
use std::process::Child;
use std::process::ChildStderr;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::str;
use std::thread;

use bytes::buf::BufMut;
use bytes::BytesMut;

use futures::Async;
use futures::future::Future;
use futures::Poll;
use futures::Sink;
use futures::stream::Stream;
use futures::sync::mpsc::channel;
use futures::try_ready;

use log::debug;
use log::error;
use log::Level::Error;
use log::log_enabled;

use tokio_codec::Decoder;


/// Spawn a process, allowing for retrieval of its output.
fn spawn(mut command: Command) -> Result<(Child, ChildStdout, ChildStderr)> {
  let mut child = command
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  // It's safe to unwrap here. `stdout` and `stderr` will always be
  // available because we configured the command this way (see above).
  let stdout = child.stdout.take().unwrap();
  let stderr = child.stderr.take().unwrap();

  Ok((child, stdout, stderr))
}


/// Send something through a sink and return on error while logging the
/// attempt.
///
/// This macro is mostly meant to be used in conjunction with
/// `futures::sync::mpsc::channel` which can only fail when the
/// receiving end is dropped.
macro_rules! send_checked {
  ($sink:ident, $data:expr) => {
    match $sink.send($data).wait() {
      Ok(sink) => sink,
      Err(_) => {
        debug!("failed to send through channel; receiving end closed?");
        return;
      }
    }
  };
}

/// Spawn a new thread that reads from stdin and passes messages back
/// using a bounded channel.
fn stream_process(command: Command) -> Result<impl Stream<Item = BytesMut, Error = IoError>> {
  /// The maximum number of `BytesMut` objects that are buffered in our channel.
  const BUFS: usize = 16;
  /// The minimum `BytesMut` capacity below which we reallocate back up
  /// to a total capacity of `BUF_MAX`.
  const BUF_MIN: usize = 4096;
  /// The maximum `BytesMut` capacity we allocate.
  const BUF_MAX: usize = 8192;

  let (mut child, mut stdout, mut stderr) = spawn(command)?;
  let (mut sink, stream) = channel(BUFS);

  thread::spawn(move || {
    let mut buf = BytesMut::with_capacity(BUF_MAX);
    loop {
      debug_assert!(buf.has_remaining_mut());

      match stdout.read(unsafe { buf.bytes_mut() }) {
        Ok(0) => {
          match child.try_wait() {
            Ok(result) => match result {
              Some(status) => {
                if !status.success() {
                  if log_enabled!(Error) {
                    let mut output = Vec::new();
                    match stderr.read_to_end(&mut output) {
                      Ok(count) => {
                        if count > 0 {
                          match str::from_utf8(&output) {
                            Ok(s) => error!("streaming process failed: {}", s),
                            Err(b) => error!("streaming process failed: {}", b),
                          }
                        }
                      },
                      Err(err) => error!("failed to read failed process' stderr: {}", err),
                    }
                  }
                  let msg = match status.code() {
                    Some(code) => format!("streaming process failed: exit code {}", code),
                    None => format!("streaming process failed"),
                  };
                  let err = IoError::new(ErrorKind::Other, msg);
                  sink = send_checked!(sink, Err(err));
                }
                return
              }
              None => debug!("read 0 bytes but process is still alive"),
            },
            // TODO: It is not quite clear whether we should continue
            //       here or break.
            Err(err) => debug!("unable to inquire process state: {}", err),
          }
        },
        Ok(n) => {
          unsafe {
            buf.advance_mut(n);
          }
          let data = buf.take();
          sink = send_checked!(sink, Ok(data));
          let cap = buf.remaining_mut();
          if cap < BUF_MIN {
            buf.reserve(BUF_MAX - cap)
          }
        },
        Err(err) => {
          sink = send_checked!(sink, Err(err));
        },
      };
    }
  });

  let stream = stream.then(|result| {
    match result {
      Ok(result) => result,
      Err(()) => {
        // A channel as used here does not produce errors (indicated
        // by the () error type). But let's play safe and not
        // introduce unnecessary panic paths by just mapping it to
        // an error that clients have to be able to deal with
        // anyway.
        Err(IoError::new(ErrorKind::Other, "unexpected channel error"))
      }
    }
  });
  Ok(stream)
}


/// A custom `Stream` implementation that marries a `Stream` over
/// `BytesMut` with a `Decoder`.
//
// Note that intuitively much of this logic should be possible to
// create using stream combinators, except for the tiny fact that all
// combinators implement `FnMut` and so we have no way to move an object
// (the decoder) into the chain.
struct Streamer<S, D> {
  stream: S,
  decoder: D,
  bytes: BytesMut,
}

impl<S, D> Streamer<S, D> {
  fn new(stream: S, decoder: D) -> Self {
    let bytes = BytesMut::new();
    Self {
      stream,
      decoder,
      bytes,
    }
  }
}

impl<S, D> Stream for Streamer<S, D>
where
  S: Stream<Item = BytesMut>,
  D: Decoder,
  D::Error: From<S::Error>,
{
  type Item = D::Item;
  type Error = D::Error;


  fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
    loop {
      match self.decoder.decode(&mut self.bytes) {
        Ok(result) => {
          if let Some(object) = result {
            return Ok(Async::Ready(Some(object)))
          }
        },
        Err(err) => return Err(err.into()),
      }

      match try_ready!(self.stream.poll()) {
        Some(read) => self.bytes.unsplit(read),
        None => return Ok(Async::Ready(None)),
      };
    }
  }
}


/// Stream data from a process and decode it on the fly.
pub fn stream_with_decoder<D>(
  command: Command,
  decoder: D,
) -> Result<impl Stream<Item = D::Item, Error = D::Error>>
where
  D: Decoder,
  D::Error: From<IoError>,
{
  // This is what a version not using a custom stream implementation
  // could look like. Unfortunately, it also doesn't work due to stupid
  // capture semantics in closures. Anyway, this logic took so long to
  // develop to begin with that it has to be kept for reference.
  // let stream = stream_process(command)?
  //   .map(|bytes| (bytes, new_decoder()))
  //   .map(|(bytes, decoder)| {
  //     unfold((bytes, decoder), |(mut bytes, mut decoder)| {
  //       decoder
  //         .decode(&mut bytes)
  //         .map_err(From::from)
  //         .transpose()
  //         .map(|result| done(result).map(|object| (object, (bytes, decoder))))
  //     })
  //   })
  //   .flatten();
  //
  // Ok(stream)

  let stream = stream_process(command)?;
  let stream = Streamer::new(stream, decoder);
  Ok(stream)
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;
  use tokio_codec::LinesCodec;


  #[test]
  fn stream_no_output() -> Result<()> {
    let command = Command::new("true");
    let future = stream_with_decoder(command, LinesCodec::new())?.collect();
    let lines = block_on_all(future)?;
    assert_eq!(lines, Vec::<String>::new());
    Ok(())
  }

  #[test]
  fn stream_multiple_lines() -> Result<()> {
    let mut command = Command::new("echo");
    command
      .arg("this is a test\nwith multiple\nlines!!!")
      .env_clear();

    let future = stream_with_decoder(command, LinesCodec::new())?.collect();
    let lines = block_on_all(future)?;
    let expected = vec![
      "this is a test".to_string(),
      "with multiple".to_string(),
      "lines!!!".to_string(),
    ];
    assert_eq!(lines, expected);
    Ok(())
  }

  #[test]
  fn stream_endless() -> Result<()> {
    let mut command = Command::new("yes");
    command.arg("yes");

    let future = stream_with_decoder(command, LinesCodec::new())?
      .take(100_000)
      .for_each(|line| {
        assert_eq!(&line, "yes");
        Ok(())
      });
    block_on_all(future)
  }

  #[test]
  fn command_setup_error() {
    let command = Command::new("/whos/your/daddy/i-dont-actually-exist");
    let result = stream_with_decoder(command, LinesCodec::new());

    match result {
      Ok(_) => panic!("streaming succeeded unexpectedly"),
      Err(err) => assert_eq!(err.kind(), ErrorKind::NotFound),
    }
  }

  #[test]
  fn command_failure() -> Result<()> {
    let command = Command::new("false");
    let future = stream_with_decoder(command, LinesCodec::new())?.collect();
    let err = block_on_all(future).unwrap_err();

    assert_eq!(&err.to_string(), "streaming process failed: exit code 1");
    Ok(())
  }
}
