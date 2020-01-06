// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use serde::Deserialize;


#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ErrorMessage {
  /// A message as provided by Polygon.
  #[serde(rename = "message")]
  pub message: String,
}

impl Display for ErrorMessage {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    write!(fmt, "{}", self.message)
  }
}


/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint.
macro_rules! Endpoint {
  ( $(#[$docs:meta])* $pub:vis $name:ident($in:ty),
    Ok => $out:ty, [$($(#[$ok_docs:meta])* $ok_status:ident,)*],
    Err => $err:ident, [$($(#[$err_docs:meta])* $err_status:ident => $variant:ident,)*]
    $($defs:tt)* ) => {

    EndpointDef! {
      $(#[$docs])* $pub $name($in),
      Ok => $out, [$($ok_status,)*],
      Err => $err, [
        /// Authentication failed for the request.
        /* 401 */ UNAUTHORIZED => AuthenticationFailed,
        $($(#[$err_docs])* $err_status => $variant,)*
      ],
      ApiErr => crate::endpoint::ErrorMessage,
      $($defs)*
    }
  };
}
