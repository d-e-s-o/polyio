// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
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
#[cfg(not(target_arch = "wasm32"))]
mod tests {
  use super::*;

  use test_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_locales() {
    let client = Client::from_env().unwrap();
    let locales = client
      .issue::<Get>(())
      .await
      .unwrap()
      .into_result()
      .unwrap();

    // We are in trouble if NYE cannot be found.
    let us = locales.iter().find(|locale| locale.locale == "US").unwrap();
    assert!(us.name.starts_with("United States"), "{}", us.name);
  }
}
