// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use serde_json::Error as JsonError;
use tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

use crate::Str;


fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
  write!(fmt, "{}", err)?;
  if let Some(src) = err.source() {
    write!(fmt, ": ")?;
    fmt_err(src, fmt)?;
  }
  Ok(())
}


/// An error type used by this crate.
#[derive(Debug)]
pub enum Error {
  /// A JSON conversion error.
  Json(JsonError),
  /// An error directly originating in this module.
  Str(Str),
  /// An URL parsing error.
  Url(ParseError),
  /// A websocket error.
  WebSocket(WebSocketError),
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Error::Json(err) => fmt_err(err, fmt),
      Error::Str(s) => write!(fmt, "{}", s),
      Error::Url(err) => fmt_err(err, fmt),
      Error::WebSocket(err) => fmt_err(err, fmt),
    }
  }
}

impl StdError for Error {}

impl From<JsonError> for Error {
  fn from(e: JsonError) -> Self {
    Error::Json(e)
  }
}

impl From<ParseError> for Error {
  fn from(e: ParseError) -> Self {
    Error::Url(e)
  }
}

impl From<WebSocketError> for Error {
  fn from(e: WebSocketError) -> Self {
    Error::WebSocket(e)
  }
}
