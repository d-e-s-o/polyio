// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;

use crate::Error;

/// The environment variable representing the API key.
const ENV_API_KEY: &str = "POLYGON_API_KEY";


/// Retrieve API related information from the environment.
///
/// This function retrieves API related information from the environment
/// and performs some preliminary validation on it. In particular, the
/// following information is retrieved:
/// - the Polygon key ID is retrieved from the POLYGON_API_KEY variable
pub fn api_info() -> Result<OsString, Error> {
  let api_key = var_os(ENV_API_KEY)
    .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_API_KEY).into()))?;

  Ok(api_key)
}
