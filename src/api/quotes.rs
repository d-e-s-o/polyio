// Copyright (C) 2020-2022 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Date;
use chrono::DateTime;
use chrono::serde::ts_milliseconds::deserialize as datetime_from_timestamp;
use chrono::Utc;
use num_decimal::Num;
use serde::Deserialize;

use crate::api::aggregates::TimeSpan;
use crate::api::response::Response;
use crate::Str;

/// Filters quote data based on the timestamp in the given direction of time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuoteTimespanFilter {
    /// Filter for data older than the given timestamp.
    LessThan,
    /// Filter for data older than, or as old as, the given timestamp.
    LessThanEqual,
    /// Filter for data newer than the given timestamp.
    GreaterThan,
    /// Filter for data newer than, or as new as, the given timestamp.
    GreaterThanEqual,
}

impl AsRef<str> for QuoteTimespanFilter {
    fn as_ref(&self) -> &'static str {
        match *self {
            QuoteTimespanFilter::LessThan => "lt",
            QuoteTimespanFilter::LessThanEqual => "lte",
            QuoteTimespanFilter::GreaterThan => "gt",
            QuoteTimespanFilter::GreaterThanEqual => "gte",
        }
    }
}

/// Specifies the sort order of the quotes, using the key specified in `sort`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuoteOrder {
    /// Order quotes oldest to newest.
    Ascending,
    /// Order quotes newest to oldests.
    Descending,
}

impl AsRef<str> for QuoteOrder {
    fn as_ref(&self) -> &'static str {
        match *self {
            QuoteOrder::Ascending => "asc",
            QuoteOrder::Descending => "desc",
        }
    }
}

/// Specifies the sort key of the quotes, especially relevant for ordering (see `QuoteOrder`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuoteSortBy {
    /// Sort quotes by timestamp.
    Timestamp,
}

impl AsRef<str> for QuoteSortBy {
    fn as_ref(&self) -> &'static str {
        match *self {
            QuoteSortBy::Timestamp => "timestamp",
        }
    }
}

/// A GET request to be made to the
/// `/v3/quotes/<symbol>` endpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct QuotesReq {
    /// The ticker symbol to request quote data for.
    pub symbol: String,
    /// A date with the format YYYY-MM-DD or a nanosecond timestamp.
    pub timestamp: String,
    /// The optional timespan filter to use.
    pub filter: Option<QuoteTimespanFilter>,
    /// The optional ordering of the quotes.
    pub order: Option<QuoteOrder>,
    /// The optional sorting of the quotes.
    pub sort: Option<QuoteSortBy>,
}


/// A ticker as returned by the
/// `/v2/aggs/ticker/<symbol>/range/1/<span>/<start>/<end>` endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Quote {
    /// The id of the exchange where this ask originates from.
    pub ask_exchange: Num,
    /// The ask price.
    pub ask_price: Num,
    /// The ask size.
    pub ask_size: Num,
    /// The id of the exchange where this bid originates from.
    pub bid_exchange: Num,
    /// The bid price.
    pub bid_price: Num,
    /// The bid size.
    pub bid_size: Num,
    #[serde(rename = "participant_timestamp")]
    pub timestamp: Num,
}

type GetResponse = Response<Option<Vec<Quote>>>;

Endpoint! {
  /// The representation of a GET request to the
  /// `/v3/quotes/<symbol>` endpoint.
  pub Get(QuotesReq),
  Ok => GetResponse, [
    /// The ticker information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(input: &Self::Input) -> Str {
    format!(
      "/v3/quotes/{sym}",
      sym = input.symbol,
    ).into()
  }
}


#[cfg(test)]
mod tests {
    use std::f64::EPSILON;
    use std::str::FromStr as _;

    use chrono::Duration;
    use chrono::NaiveDate;
    use chrono::TimeZone as _;
    use serde_json::from_str as from_json;

    #[cfg(not(target_arch = "wasm32"))]
    use test_log::test;

    #[cfg(not(target_arch = "wasm32"))]
    use crate::Client;

    use super::*;

    /// Make sure that we can deserialize an `Quote`.
    #[test]
    fn deserialize_quote() {
        let response = r#"{
  "ask_exchange": 11,
  "ask_price": 162.95,
  "ask_size": 4,
  "bid_exchange": 11,
  "bid_price": 162.93,
  "bid_size": 1,
  "participant_timestamp": 1646441556665720000,
  "sequence_number": 115884766,
  "sip_timestamp": 1646441556665973800,
  "tape": 3
}"#;

        let quote = from_json::<Quote>(response).unwrap();

        assert_eq!(quote.ask_exchange, Num::new(11, 1));
        assert_eq!(quote.ask_price, Num::new(16295, 100));
        assert_eq!(quote.ask_size, Num::new(4, 1));
        assert_eq!(quote.bid_exchange, Num::new(11, 1));
        assert_eq!(quote.bid_price, Num::new(16293, 100));
        assert_eq!(quote.bid_size, Num::new(1, 1));
    }

    #[test]
    fn deserialize_response() {
        let response = r#"{
  "results": [
    {
      "ask_exchange": 11,
      "ask_price": 162.95,
      "ask_size": 4,
      "bid_exchange": 11,
      "bid_price": 162.93,
      "bid_size": 1,
      "participant_timestamp": 1646441556665720000,
      "sequence_number": 115884766,
      "sip_timestamp": 1646441556665973800,
      "tape": 3
    }
  ],
  "status": "OK",
  "request_id": "foo",
  "next_url": "https://api.polygon.io/v3/quotes/AAPL?cursor=foo"
}"#;

        let mut quotes = from_json::<GetResponse>(response)
            .unwrap()
            .into_result()
            .unwrap()
            .unwrap();

        assert_eq!(quotes.len(), 1);

        let quote = quotes.remove(0);

        assert_eq!(quote.bid_price, Num::new(16293, 100));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test(tokio::test)]
    async fn request_aapl_day_aggregates() {
        let start = Utc.from_utc_date(&NaiveDate::from_str("2022-02-01").unwrap());
        let end = Utc.from_utc_date(&NaiveDate::from_str("2022-02-28").unwrap());

        let client = Client::from_env().unwrap();
        let request = QuotesReq {
            symbol: "AAPL".into(),
            timestamp: "2022-02-01".into(),
            filter: Some(QuoteTimespanFilter::LessThan),
            order: Some(QuoteOrder::Descending),
            sort: None,
        };

        let quotes = client
            .issue::<Get>(request)
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .unwrap();

        // The number of trading days was inferred to be 21.
        //assert_eq!(aggregates.len(), 21);
        dbg!(quotes);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test(tokio::test)]
    async fn request_non_existent_quotes() {
        let client = Client::from_env().unwrap();
        let today = Utc::today();
        let request = QuotesReq {
            symbol: "SPWRX".into(),
            timestamp: "2022-02-01".into(),
            filter: None,
            order: None,
            sort: None,
        };

        let quotes = client
            .issue::<Get>(request)
            .await
            .unwrap()
            .into_result()
            .unwrap();

        assert_eq!(&*quotes.unwrap(), &*vec!());
    }
}
