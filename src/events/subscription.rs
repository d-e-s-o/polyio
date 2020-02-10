// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::Str;


/// Possible subscriptions for a stock.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

impl Subscription {
  /// Retrieve the `Stock` object common to all variants in a
  /// `Subscription`.
  pub fn stock(&self) -> &Stock {
    match self {
      Subscription::SecondAggregates(stock)
      | Subscription::MinuteAggregates(stock)
      | Subscription::Trades(stock)
      | Subscription::Quotes(stock) => &stock,
    }
  }
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
