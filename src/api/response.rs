// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use crate::error::Error;


/// The response as returned by various endpoints.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Response<T> {
  /// The status message associated with this response.
  #[serde(rename = "status")]
  status: String,
  /// The actual result.
  #[serde(rename = "results")]
  result: T,
}

impl<T> Response<T> {
  /// Convert a `Response` into a `Result`.
  pub fn into_result(self) -> Result<T, Error> {
    match self.status.as_ref() {
      "OK" => Ok(self.result),
      status => {
        let err = format!("response did not indicate success: {}", status);
        Err(Error::Str(err.into()))
      },
    }
  }
}
