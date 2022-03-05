// Copyright (C) 2020-2022 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use thiserror::Error;


/// A response error as reported by Polygon.
#[derive(Clone, Debug, PartialEq, Error)]
#[error("response did not indicate success: {0}")]
pub struct ResponseError(pub String);


/// The response as returned by various endpoints.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "status", content = "results")]
pub enum Response<T> {
  /// The request was successful and all results were retrieved.
  #[serde(rename = "OK")]
  Ok(T),
  /// The response contains data that was delayed and does not contain
  /// the most recent data points.
  #[serde(rename = "DELAYED")]
  Delayed(T),
  /// An error occurred or unexpected status was reported.
  #[serde(other)]
  Err,
}

impl<T> Response<T> {
  /// Convert a `Response` into a `Result`.
  ///
  /// Both `Ok` and `Delayed` variants are treated as success.
  pub fn into_result(self) -> Result<T, ResponseError> {
    match self {
      Self::Ok(data) | Self::Delayed(data) => Ok(data),
      Self::Err => Err(ResponseError("an unexpected status was reported".into())),
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;


  /// Check that we can decode an Ok response.
  #[test]
  fn decode_ok() {
    let json = r#"{"status":"OK","results":["abc"]}"#;
    let response = from_json::<Response<Vec<String>>>(json).unwrap();
    match response {
      Response::Ok(data) if data.as_slice() == ["abc"] => (),
      _ => panic!("unexpected result"),
    }
  }

  /// Check that we can decode a delayed response.
  #[test]
  fn decode_delayed() {
    let json = r#"{"status":"DELAYED","results":["abc"]}"#;
    let response = from_json::<Response<Vec<String>>>(json).unwrap();
    match response {
      Response::Delayed(data) if data.as_slice() == ["abc"] => (),
      _ => panic!("unexpected result"),
    }
  }
}
