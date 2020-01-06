// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
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


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use test_env_log::test;

  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn request_markets() -> Result<(), Error> {
    let client = Client::from_env()?;
    let markets = client
      .issue::<Get>(())
      .await
      .map_err(EndpointError::from)?
      .into_result()?;

    // We are in trouble if NYE cannot be found.
    let stocks = markets
      .iter()
      .find(|market| market.market == "STOCKS")
      .unwrap();
    assert_eq!(stocks.description, "Stocks / Equities / ETFs");
    Ok(())
  }
}
