// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use crate::api::response::Response;
use crate::Str;


/// An enum describing the ticker's market.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Market {
  /// The stock market.
  #[serde(rename = "STOCKS")]
  Stocks,
  /// The indices market.
  #[serde(rename = "INDEX")]
  Indices,
  /// The foreign exchange market.
  #[serde(rename = "FX")]
  ForeignExchange,
}

impl AsRef<str> for Market {
  fn as_ref(&self) -> &'static str {
    match *self {
      Market::Stocks => "STOCKS",
      Market::Indices => "INDEX",
      Market::ForeignExchange => "FX",
    }
  }
}


/// The type of a ticker.
///
/// Please note that not all types are made available, as the reference
/// set of types can be inquired dynamically.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Type {
  /// The ticker represents a common stock.
  #[serde(rename = "CS")]
  CommonStock,
  /// Any other type.
  #[serde(other)]
  Other,
}


/// A ticker as returned by the /v2/reference/tickers/<ticker> endpoint.
///
/// Please note that not all fields available in a request are
/// represented here.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Ticker {
  /// The ticker.
  #[serde(rename = "ticker")]
  pub ticker: String,
  /// The ticker's name.
  #[serde(rename = "name")]
  pub name: String,
  /// The ticker's name.
  #[serde(rename = "market")]
  pub market: Market,
  /// The locale.
  #[serde(rename = "locale")]
  pub locale: String,
  /// The ticker's currency.
  #[serde(rename = "currency")]
  pub currency: String,
  /// Whether the ticker is still active.
  #[serde(rename = "active")]
  pub active: bool,
  /// The ticker's type.
  #[serde(rename = "type")]
  pub type_: Option<Type>,
}


/// A ticker as returned by the /v2/reference/tickers/<ticker> endpoint.
///
/// Please note that not all fields available in a request are
/// represented here.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TickerResp {
  /// The ticker information.
  #[serde(rename = "ticker")]
  pub ticker: Ticker,
}


Endpoint! {
  /// The representation of a GET request to the /v2/reference/tickers/<ticker> endpoint.
  pub Get(String),
  Ok => Response<TickerResp>, [
    /// The ticker information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// The specified resource was not found.
    ///
    /// This error will also occur on valid tickers when the market is
    /// closed.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/reference/tickers/{}", input).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn request_aapl_ticker() -> Result<(), Error> {
    let client = Client::from_env()?;
    let result = client.issue::<Get>("AAPL".into()).await;

    match result {
      Ok(response) => {
        let aapl = response.into_result()?.ticker;
        assert_eq!(aapl.ticker, "AAPL");
        assert_eq!(aapl.name, "Apple Inc");
        assert_eq!(aapl.market, Market::Stocks);
        assert_eq!(aapl.locale, "US");
        assert_eq!(aapl.currency, "USD");
      },
      Err(GetError::NotFound(..)) => (),
      Err(..) => panic!("unexpected error: {:?}", result),
    }
    Ok(())
  }
}
