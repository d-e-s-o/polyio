// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use crate::api::response::Response;
use crate::Str;


/// A locale as returned by the /v2/reference/markets endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Market {
  /// The market.
  #[serde(rename = "market")]
  pub market: String,
  /// A description of the market.
  #[serde(rename = "desc")]
  pub description: String,
}


Endpoint! {
  /// The representation of a GET request to the /v2/reference/markets endpoint.
  pub Get(()),
  Ok => Response<Vec<Market>>, [
    /// The market information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/reference/markets".into()
  }
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use super::*;

  use test_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_markets() {
    let client = Client::from_env().unwrap();
    let markets = client
      .issue::<Get>(())
      .await
      .unwrap()
      .into_result()
      .unwrap();

    // We are in trouble if NYE cannot be found.
    let stocks = markets
      .iter()
      .find(|market| market.market == "STOCKS")
      .unwrap();
    assert_eq!(stocks.description, "Stocks / Equities / ETFs");
  }
}
