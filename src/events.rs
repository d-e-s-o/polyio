// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module represents the integration point between our Python
//! streaming script and Rust. It allows for conveniently spawning a
//! process streaming the data and converting the results into proper
//! Rust event objects.

use std::env::var_os;
use std::error::Error as StdError;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::process::Command;

use futures::stream::Stream;

use serde::Deserialize;
use serde_json::Error as JsonError;
use serde_json::from_str as from_json;

use tokio_codec::LinesCodec;

use crate::stock::Aggregate;
use crate::stock::Quote;
use crate::stock::Trade;
use crate::Str;
use crate::stream::stream_with_decoder;

/// The Python script connecting to the Polygon API and streaming
/// events. We assume it is reachable through the PATH environment
/// variable.
const POLYGON_STREAM: &str = "polygon-stream";
/// The environment variable defining Python's module search path.
const PYTHONPATH: &str = "PYTHONPATH";
/// The environment variable holding the API key for Polygon.
const POLYGON_API_KEY: &str = "POLYGON_API_KEY";


fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
  write!(fmt, "{}", err)?;
  if let Some(src) = err.source() {
    write!(fmt, ": ")?;
    fmt_err(src, fmt)?;
  }
  Ok(())
}


#[derive(Debug)]
pub enum EventError {
  Io(IoError),
  Json(JsonError),
}

impl Display for EventError {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      EventError::Io(err) => fmt_err(err, fmt),
      EventError::Json(err) => fmt_err(err, fmt),
    }
  }
}

impl StdError for EventError {}

impl From<IoError> for EventError {
  fn from(e: IoError) -> Self {
    EventError::Io(e)
  }
}


/// Possible subscriptions for a stock.
#[derive(Clone, Debug, PartialEq)]
pub enum Stock {
  /// Subscribe to the stock with the given symbol.
  Symbol(Str),
  /// Subscribe to an event type for all available stocks.
  All,
}

impl Display for Stock {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Stock::Symbol(symbol) => write!(fmt, "{}", symbol),
      Stock::All => write!(fmt, "*"),
    }
  }
}


/// An enum describing a subscription.
#[derive(Clone, Debug, PartialEq)]
pub enum Subscription {
  /// A type representing second aggregates for the given stock.
  SecondAggregates(Stock),
  /// A type representing minute aggregates for the given stock.
  MinuteAggregates(Stock),
  /// A type representing trades for the given stock.
  Trades(Stock),
  /// A type representing quotes for the given stock.
  Quotes(Stock),
}

impl Display for Subscription {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Subscription::SecondAggregates(stock) => write!(fmt, "A.{}", stock.to_string()),
      Subscription::MinuteAggregates(stock) => write!(fmt, "AM.{}", stock.to_string()),
      Subscription::Trades(stock) => write!(fmt, "T.{}", stock.to_string()),
      Subscription::Quotes(stock) => write!(fmt, "Q.{}", stock.to_string()),
    }
  }
}


/// An enum representing the type of event we received from Polygon.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "ev")]
pub enum Event {
  /// A tick for a second aggregate for a stock.
  #[serde(rename = "A")]
  SecondAggregate(Aggregate),
  /// A tick for a minute aggregate for a stock.
  #[serde(rename = "AM")]
  MinuteAggregate(Aggregate),
  /// A tick for a trade of a stock.
  #[serde(rename = "T")]
  Trade(Trade),
  /// A tick for a quote for a stock.
  #[serde(rename = "Q")]
  Quote(Quote),
}


/// Create a command that stream data from the Polygon service.
fn polygon_command<'s, S>(api_key: &OsStr, subscriptions: S) -> Command
where
  S: IntoIterator<Item = &'s Subscription>,
{
  let mut command = Command::new(POLYGON_STREAM);
  command
    .env_clear()
    .env(POLYGON_API_KEY, api_key)
    .args(subscriptions.into_iter().map(|sub| sub.to_string()));

  if let Some(path) = var_os(PYTHONPATH) {
    command.env(PYTHONPATH, path);
  }

  command
}


/// Stream events from a command.
fn stream_events(command: Command) -> IoResult<impl Stream<Item = Event, Error = EventError>> {
  // It is rather hard & costly (in terms of performance) to write a
  // generic JSON decoder based solely on `BufMut` objects. To cope with
  // this problem we force each object onto a single line. Hence, we can
  // be sure that, upon receiving a line, we also received an object
  // that should be decodable.
  let stream = stream_with_decoder(command, LinesCodec::new())?
    .map_err(EventError::Io)
    .and_then(|line| from_json(&line).map_err(EventError::Json));

  Ok(stream)
}


/// Subscribe to and stream events from the Polygon service.
pub fn subscribe<'s, S>(
  api_key: &OsStr,
  subscriptions: S,
) -> IoResult<impl Stream<Item = Event, Error = EventError>>
where
  S: IntoIterator<Item = &'s Subscription>,
{
  let command = polygon_command(api_key, subscriptions);
  stream_events(command)
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::os::unix::ffi::OsStrExt;

  use tokio::runtime::current_thread::block_on_all;


  #[test]
  fn parse_event() {
    let response = r#"{
      "ev": "AM",
      "sym": "MSFT",
      "v": 10204,
      "av": 200304,
      "op": 114.04,
      "vw": 114.4040,
      "o": 114.11,
      "c": 114.14,
      "h": 114.19,
      "l": 114.09,
      "a": 114.1314,
      "s": 1536036818784,
      "e": 1536036818784
    }"#;

    let event = from_json::<Event>(&response).unwrap();
    match event {
      Event::MinuteAggregate(aggregate) => {
        assert_eq!(aggregate.symbol, "MSFT");
      },
      _ => panic!("unexpected event: {:?}", event),
    }
  }

  #[test]
  fn stream_dummy() -> Result<(), EventError> {
    // We could `cat` the file directly, but including the data like
    // this makes the test independent of the file. It also is
    // potentially friendlier when debugging and does not introduce a
    // dependency to yet another program (we already use `echo`
    // elsewhere).
    let data = include_bytes!("events.test.dat");
    let mut command = Command::new("echo");
    command.arg("-n").arg(OsStr::from_bytes(data));

    let future = stream_events(command)?.collect();
    let events = block_on_all(future)?;

    assert_eq!(events.len(), 24);
    Ok(())
  }
}
