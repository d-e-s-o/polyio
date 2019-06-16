// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::Str;


/// An error type used by this crate.
#[derive(Debug)]
pub enum Error {
  /// An error directly originating in this module.
  Str(Str),
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Error::Str(s) => write!(fmt, "{}", s),
    }
  }
}

impl StdError for Error {}
