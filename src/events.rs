// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::time::SystemTime;

use futures::stream::once;
use futures::stream::Stream;

use num_decimal::Num;

use serde::Deserialize;
use serde_json::Error as JsonError;

use crate::stock::Aggregate;
use crate::stock::Quote;
use crate::stock::Trade;
use crate::Str;


fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
  write!(fmt, "{}", err)?;
  if let Some(src) = err.source() {
    write!(fmt, ": ")?;
    fmt_err(src, fmt)?;
  }
  Ok(())
}


#[derive(Debug)]
pub enum EventError {
  Io(IoError),
  Json(JsonError),
}

impl Display for EventError {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      EventError::Io(err) => fmt_err(err, fmt),
      EventError::Json(err) => fmt_err(err, fmt),
    }
  }
}

impl StdError for EventError {}

impl From<IoError> for EventError {
  fn from(e: IoError) -> Self {
    EventError::Io(e)
  }
}


/// Possible subscriptions for a stock.
#[derive(Clone, Debug, PartialEq)]
pub enum Stock {
  /// Subscribe to the stock with the given symbol.
  Symbol(Str),
  /// Subscribe to an event type for all available stocks.
  All,
}

impl Display for Stock {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Stock::Symbol(symbol) => write!(fmt, "{}", symbol),
      Stock::All => write!(fmt, "*"),
    }
  }
}


/// An enum describing a subscription.
#[derive(Clone, Debug, PartialEq)]
pub enum Subscription {
  /// A type representing second aggregates for the given stock.
  SecondAggregates(Stock),
  /// A type representing minute aggregates for the given stock.
  MinuteAggregates(Stock),
  /// A type representing trades for the given stock.
  Trades(Stock),
  /// A type representing quotes for the given stock.
  Quotes(Stock),
}

impl Display for Subscription {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Subscription::SecondAggregates(stock) => write!(fmt, "A.{}", stock.to_string()),
      Subscription::MinuteAggregates(stock) => write!(fmt, "AM.{}", stock.to_string()),
      Subscription::Trades(stock) => write!(fmt, "T.{}", stock.to_string()),
      Subscription::Quotes(stock) => write!(fmt, "Q.{}", stock.to_string()),
    }
  }
}


/// An enum representing the type of event we received from Polygon.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "ev")]
pub enum Event {
  /// A tick for a second aggregate for a stock.
  #[serde(rename = "A")]
  SecondAggregate(Aggregate),
  /// A tick for a minute aggregate for a stock.
  #[serde(rename = "AM")]
  MinuteAggregate(Aggregate),
  /// A tick for a trade of a stock.
  #[serde(rename = "T")]
  Trade(Trade),
  /// A tick for a quote for a stock.
  #[serde(rename = "Q")]
  Quote(Quote),
}


/// Subscribe to and stream events from the Polygon service.
pub fn subscribe<'s, S>(
  api_key: &OsStr,
  subscriptions: S,
) -> IoResult<impl Stream<Item = Event, Error = EventError>>
where
  S: IntoIterator<Item = &'s Subscription>,
{
  // TODO: Right now we just return a stream with a single dummy trade.
  //       We have to return something (as opposed to just not
  //       implementing it), otherwise Rust cannot deduce the return
  //       type properly.
  let trade = Trade {
    symbol: "MSFT".to_string(),
    exchange: 4,
    price: Num::from_int(100),
    quantity: 1,
    conditions: Vec::new(),
    timestamp: SystemTime::now(),
  };
  let event = Event::Trade(trade);
  Ok(once(Ok(event)))
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::os::unix::ffi::OsStrExt;

  use serde_json::from_str as from_json;

  use tokio::runtime::current_thread::block_on_all;


  #[test]
  fn parse_event() {
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

    let event = from_json::<Event>(&response).unwrap();
    match event {
      Event::MinuteAggregate(aggregate) => {
        assert_eq!(aggregate.symbol, "MSFT");
      },
      _ => panic!("unexpected event: {:?}", event),
    }
  }
}
