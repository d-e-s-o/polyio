// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use num_decimal::Num;

use serde::de::Deserializer;
use serde::Deserialize;


/// Deserialize a time stamp as a `SystemTime`.
fn system_time<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  let ms = u64::deserialize(deserializer)?;
  let duration = Duration::from_millis(ms);
  Ok(UNIX_EPOCH + duration)
}


/// A data point for a trade.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Trade {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The exchange the trade occurred on.
  #[serde(rename = "x")]
  pub exchange: u64,
  /// The price.
  #[serde(rename = "p")]
  pub price: Num,
  /// The number of shares traded.
  #[serde(rename = "s")]
  pub quantity: u64,
  /// The trade conditions.
  #[serde(rename = "c")]
  pub conditions: Vec<u64>,
  /// The trade's timestamp (in UNIX milliseconds).
  #[serde(rename = "t", deserialize_with = "system_time")]
  pub timestamp: SystemTime,
}


/// A quote for a stock.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Quote {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The exchange where the stock is being asked for
  #[serde(rename = "bx")]
  pub bid_exchange: u64,
  /// The bid price.
  #[serde(rename = "bp")]
  pub bid_price: Num,
  /// The bid quantity
  #[serde(rename = "bs")]
  pub bid_quantity: u64,
  /// The exchange the trade occurred on.
  #[serde(rename = "ax")]
  pub ask_exchange: u64,
  /// The ask price.
  #[serde(rename = "ap")]
  pub ask_price: Num,
  /// The bid quantity
  #[serde(rename = "as")]
  pub ask_quantity: u64,
  /// The quote condition.
  #[serde(rename = "c")]
  pub condition: u64,
  /// The quote's timestamp (in UNIX milliseconds).
  #[serde(rename = "t", deserialize_with = "system_time")]
  pub timestamp: SystemTime,
}


/// An aggregate for a stock.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Aggregate {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The tick volume.
  #[serde(rename = "v")]
  pub volume: u64,
  /// The accumulated volume.
  #[serde(rename = "av")]
  pub accumulated_volume: u64,
  /// Today's official opening price.
  #[serde(rename = "op")]
  pub open_price_today: Num,
  /// Volume weighted average price.
  #[serde(rename = "vw")]
  pub volume_weighted_average_price: Num,
  /// The tick's open price.
  #[serde(rename = "o")]
  pub open_price: Num,
  /// The tick's close price.
  #[serde(rename = "c")]
  pub close_price: Num,
  /// The tick's high price.
  #[serde(rename = "h")]
  pub high_price: Num,
  /// The tick's low price.
  #[serde(rename = "l")]
  pub low_price: Num,
  /// The tick's average price divided by the volume weighted average
  /// price.
  #[serde(rename = "a")]
  pub average_price: Num,
  /// The tick's start timestamp (in UNIX milliseconds).
  #[serde(rename = "s", deserialize_with = "system_time")]
  pub start_timestamp: SystemTime,
  /// The tick's end timestamp (in UNIX milliseconds).
  #[serde(rename = "e", deserialize_with = "system_time")]
  pub end_timestamp: SystemTime,
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;


  #[test]
  fn parse_trade() {
    let response = r#"{
      "ev": "T",
      "sym": "MSFT",
      "x": 4,
      "p": 114.125,
      "s": 100,
      "c": [0, 12],
      "t": 1536036818784
    }"#;
    let trade = from_json::<Trade>(&response).unwrap();
    assert_eq!(trade.symbol, "MSFT");
    assert_eq!(trade.exchange, 4);
    assert_eq!(trade.price, Num::new(114125, 1000));
    assert_eq!(trade.quantity, 100);
    assert_eq!(trade.conditions, vec![0, 12]);
    assert_eq!(
      trade.timestamp,
      UNIX_EPOCH + Duration::from_millis(1536036818784),
    );
  }

  #[test]
  fn parse_quote() {
    let response = r#"{
      "ev": "Q",
      "sym": "MSFT",
      "bx": 4,
      "bp": 114.125,
      "bs": 100,
      "ax": 7,
      "ap": 114.128,
      "as": 160,
      "c": 0,
      "t": 1536036818784
    }"#;
    let quote = from_json::<Quote>(&response).unwrap();
    assert_eq!(quote.symbol, "MSFT");
    assert_eq!(quote.bid_exchange, 4);
    assert_eq!(quote.bid_price, Num::new(114125, 1000));
    assert_eq!(quote.bid_quantity, 100);
    assert_eq!(quote.ask_exchange, 7);
    assert_eq!(quote.ask_price, Num::new(114128, 1000));
    assert_eq!(quote.ask_quantity, 160);
    assert_eq!(quote.condition, 0);
    assert_eq!(
      quote.timestamp,
      UNIX_EPOCH + Duration::from_millis(1536036818784),
    );
  }

  #[test]
  fn parse_aggregate() {
    let response = r#"{
      "ev": "AM",
      "sym": "MSFT",
      "v": 10204,
      "av": 200304,
      "op": 114.04,
      "vw": 114.4040,
      "o": 114.11,
      "c": 114.14,
      "h": 114.19,
      "l": 114.09,
      "a": 114.1314,
      "s": 1536036818784,
      "e": 1536036818784
    }"#;

    let aggregate = from_json::<Aggregate>(&response).unwrap();
    assert_eq!(aggregate.symbol, "MSFT");
    assert_eq!(aggregate.volume, 10204);
    assert_eq!(aggregate.accumulated_volume, 200304);
    assert_eq!(aggregate.open_price_today, Num::new(11404, 100));
    assert_eq!(
      aggregate.volume_weighted_average_price,
      Num::new(1144040, 10000),
    );
    assert_eq!(aggregate.open_price, Num::new(11411, 100));
    assert_eq!(aggregate.close_price, Num::new(11414, 100));
    assert_eq!(aggregate.high_price, Num::new(11419, 100));
    assert_eq!(aggregate.low_price, Num::new(11409, 100));
    assert_eq!(aggregate.average_price, Num::new(1141314, 10000));
    assert_eq!(
      aggregate.start_timestamp,
      UNIX_EPOCH + Duration::from_millis(1536036818784),
    );
    assert_eq!(
      aggregate.end_timestamp,
      UNIX_EPOCH + Duration::from_millis(1536036818784),
    );
  }
}
