// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use crate::api::response::Response;
use crate::Str;


/// A locale as returned by the /v2/reference/locales endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Locale {
  /// The locale.
  #[serde(rename = "locale")]
  pub locale: String,
  /// The name of the locale.
  #[serde(rename = "name")]
  pub name: String,
}


Endpoint! {
  /// The representation of a GET request to the /v2/reference/locales endpoint.
  pub Get(()),
  Ok => Response<Vec<Locale>>, [
    /// The locales information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/reference/locales".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use test_env_log::test;

  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn request_locales() -> Result<(), Error> {
    let client = Client::from_env()?;
    let locales = client
      .issue::<Get>(())
      .await
      .map_err(EndpointError::from)?
      .into_result()?;

    // We are in trouble if NYE cannot be found.
    let us = locales.iter().find(|locale| locale.locale == "US").unwrap();
    assert!(us.name.starts_with("United States"), "{}", us.name);
    Ok(())
  }
}
