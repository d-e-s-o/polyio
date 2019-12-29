// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;

use url::Url;

use crate::Error;

/// The base URL to the market data stream to use.
const ENV_STREAM_URL: &str = "POLYGON_STREAM_URL";
/// The environment variable representing the API key.
const ENV_API_KEY: &str = "POLYGON_API_KEY";

/// The default stream URL.
const DEFAULT_STREAM_URL: &str = "wss://socket.polygon.io";


/// An object encapsulating the information used for working with the
/// Alpaca API.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiInfo {
  /// The base URL for market data streaming.
  pub(crate) stream_url: Url,
  /// The API key to use for authentication.
  pub(crate) api_key: String,
}

impl ApiInfo {
  /// Create an `ApiInfo` object with information from the environment.
  ///
  /// This constructor retrieves API related information from the
  /// environment and performs some preliminary validation on it. The
  /// following information is used:
  /// - the Polygon streaming base URL is retrieved from the
  ///   POLYGON_STREAM_URL variable
  /// - the Polygon API key is retrieved from the POLYGON_API_KEY
  ///   variable
  pub fn from_env() -> Result<Self, Error> {
    let stream_url = var_os(ENV_STREAM_URL)
      .unwrap_or_else(|| OsString::from(DEFAULT_STREAM_URL))
      .into_string()
      .map_err(|_| {
        Error::Str(
          format!(
            "{} environment variable is not a valid string",
            ENV_STREAM_URL
          )
          .into(),
        )
      })?;
    let stream_url = Url::parse(&stream_url)?;

    let api_key = var_os(ENV_API_KEY)
      .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_API_KEY).into()))?
      .into_string()
      .map_err(|_| {
        Error::Str(format!("{} environment variable is not a valid string", ENV_API_KEY).into())
      })?;

    Ok(Self {
      stream_url,
      api_key,
    })
  }
}
