// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use thiserror::Error;


/// A response error as reported by Polygon.
#[derive(Clone, Debug, PartialEq, Error)]
#[error("response did not indicate success: {0}")]
pub struct ResponseError(pub String);


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
  pub fn into_result(self) -> Result<T, ResponseError> {
    match self.status.as_ref() {
      // On the free trier data for the current day may be delayed. We
      // just handle that transparently and like a success.
      "OK" | "DELAYED" => Ok(self.result),
      _ => Err(ResponseError(self.status)),
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;


  #[test]
  fn success() {
    let response = Response {
      status: "OK".into(),
      result: 42,
    };

    assert_eq!(response.into_result().unwrap(), 42);
  }


  /// Check that we can handle a "delayed" response correctly.
  #[test]
  fn delayed() {
    let response = Response {
      status: "DELAYED".into(),
      result: "foobar",
    };

    assert_eq!(response.into_result().unwrap(), "foobar");
  }


  #[test]
  fn error() {
    let response = Response {
      status: "ERR".into(),
      result: (),
    };

    let err = response.into_result().unwrap_err();
    assert_eq!(err, ResponseError("ERR".into()));
    assert_eq!(&err.to_string(), "response did not indicate success: ERR");
  }
}
