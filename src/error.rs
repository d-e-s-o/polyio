// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::str::from_utf8;

use http::Error as HttpError;
use http::StatusCode as HttpStatusCode;
use http_endpoint::Error as EndpointError;

#[cfg(not(target_arch = "wasm32"))]
use hyper::Error as HyperError;
use serde_json::Error as JsonError;
use thiserror::Error as ThisError;
#[cfg(not(target_arch = "wasm32"))]
use tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

use crate::Str;


/// An error encountered while issuing a request.
#[derive(Debug, ThisError)]
pub enum RequestError<E>
where
  E: Debug + Display,
{
  /// An endpoint reported error.
  #[error("the endpoint reported an error")]
  Endpoint(E),
  /// An error reported by the `hyper` crate.
  #[cfg(not(target_arch = "wasm32"))]
  #[error("the hyper crate reported an error")]
  Hyper(
    #[from]
    #[source]
    HyperError,
  ),
}


#[derive(Clone, Debug, ThisError)]
pub struct HttpBody(Vec<u8>);

impl Display for HttpBody {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match from_utf8(&self.0) {
      Ok(s) => fmt.write_str(s)?,
      Err(b) => write!(fmt, "{:?}", b)?,
    }
    Ok(())
  }
}


/// An error type used by this crate.
#[derive(Debug, ThisError)]
pub enum Error {
  /// An HTTP related error.
  #[error("encountered an HTTP related error")]
  Http(#[source] HttpError),
  /// We encountered an HTTP that either represents a failure or is not
  /// supported.
  #[error("encountered an unexpected HTTP status: {0}")]
  HttpStatus(HttpStatusCode, #[source] HttpBody),
  /// A JSON conversion error.
  #[error("a JSON conversion failed")]
  Json(
    #[from]
    #[source]
    JsonError,
  ),
  /// An error directly originating in this module.
  #[error("{0}")]
  Str(Str),
  /// An URL parsing error.
  #[error("failed to parse the URL")]
  Url(
    #[from]
    #[source]
    ParseError,
  ),
  /// A websocket error.
  #[cfg(not(target_arch = "wasm32"))]
  #[error("encountered a websocket related error")]
  WebSocket(
    #[from]
    #[source]
    WebSocketError,
  ),
}

impl From<EndpointError> for Error {
  fn from(src: EndpointError) -> Self {
    match src {
      EndpointError::Http(err) => Error::Http(err),
      EndpointError::HttpStatus(status, data) => Error::HttpStatus(status, HttpBody(data)),
      EndpointError::Json(err) => Error::Json(err),
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::error::Error as _;


  /// Check that textual error representations are as expected.
  #[test]
  fn str_errors() {
    let err = Error::Str("foobar failed".into());
    assert_eq!(err.to_string(), "foobar failed");

    let err = Error::from(ParseError::EmptyHost);
    assert_eq!(err.to_string(), "failed to parse the URL");
    assert_eq!(
      err.source().unwrap().to_string(),
      ParseError::EmptyHost.to_string()
    );

    let status = HttpStatusCode::from_u16(404).unwrap();
    let body = HttpBody(b"entity not available".to_vec());
    let err = Error::HttpStatus(status, body);
    assert_eq!(
      err.to_string(),
      "encountered an unexpected HTTP status: 404 Not Found"
    );
    assert_eq!(err.source().unwrap().to_string(), "entity not available");
  }
}
