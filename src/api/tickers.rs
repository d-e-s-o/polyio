// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use url::form_urlencoded::Serializer;

use crate::api::ticker;
use crate::Error;
use crate::Str;


/// A GET request to be made to the /v2/reference/tickers endpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickerReq {
  /// The market a ticker should be traded on.
  pub market: Option<ticker::Market>,
  /// The type to filter for.
  pub type_: Option<ticker::Type>,
  /// Whether to return only active/inactive tickers.
  pub active: Option<bool>,
  /// The page to show.
  ///
  /// Pages start at 1.
  pub page: usize,
}

impl Default for TickerReq {
  fn default() -> Self {
    Self {
      market: None,
      type_: None,
      active: None,
      page: 1,
    }
  }
}


/// An object representing a single page of a response.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Page {
  /// The status message associated with this response.
  #[serde(rename = "status")]
  status: String,
  /// The page being retrieved.
  #[serde(rename = "page")]
  page: usize,
  /// The maximum number of results contained in one page.
  #[serde(rename = "perPage")]
  per_page: usize,
  /// The total result count
  #[serde(rename = "count")]
  count: usize,
  /// The actual tickers.
  #[serde(rename = "tickers")]
  tickers: Vec<ticker::Ticker>,
}

impl Page {
  /// Convert a `Page` into a `Result`.
  pub fn into_result(self) -> Result<Vec<ticker::Ticker>, Error> {
    match self.status.as_ref() {
      "OK" => Ok(self.tickers),
      status => {
        let err = format!("response did not indicate success: {}", status);
        Err(Error::Str(err.into()))
      },
    }
  }
}


Endpoint! {
  /// The representation of a GET request to the /v2/reference/tickers endpoint.
  pub Get(TickerReq),
  Ok => Page, [
    /// The ticker information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// The specified resource was not found.
    /* 404 */ NOT_FOUND => NotFound,
    /// A parameter is invalid or incorrect.
    /* 409 */ CONFLICT => InvalidRequest,
  ]

  fn path(_input: &Self::Input) -> Str {
    "/v2/reference/tickers".into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    let mut query = Serializer::new(String::new());
    if let Some(market) = input.market {
      query.append_pair("market", market.as_ref());
    }
    if let Some(type_) = input.type_ {
      if let Some(s) = type_.as_str() {
        query.append_pair("type", s);
      }
    }
    if let Some(active) = input.active {
      query.append_pair("active", if active { "true" } else { "false" });
    }

    query.append_pair("sort", "ticker");
    query.append_pair("page", &input.page.to_string());

    Some(query.finish().into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_tickers() {
    let client = Client::from_env().unwrap();
    let mut page = 1;

    loop {
      let request = TickerReq {
        active: Some(true),
        market: Some(ticker::Market::Stocks),
        page,
        ..Default::default()
      };

      let response = client.issue::<Get>(request).await.unwrap();
      assert_eq!(response.page, page);

      let tickers = response.into_result().unwrap();
      assert!(tickers.len() > 0);

      // Let's hope that AAPL sticks around for a while.
      if let Some(aapl) = tickers.iter().find(|ticker| ticker.ticker == "AAPL") {
        assert_eq!(aapl.name, "Apple Inc. Common Stock");
        assert_eq!(aapl.market, ticker::Market::Stocks);
        assert_eq!(aapl.locale, "US");
        assert_eq!(aapl.currency, "USD");
        assert!(aapl.active);
        break
      }

      page += 1;
    }
  }
}
