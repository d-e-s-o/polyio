// Copyright (C) 2020-2022 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::serde::ts_milliseconds::deserialize as datetime_from_timestamp;
use chrono::Date;
use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;

use crate::api::response::Response;
use crate::Str;


/// An enumeration of the various supported time span values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeSpan {
  /// A minutely aggregate.
  Minute,
  /// A hourly aggregate.
  Hour,
  /// A daily aggregate.
  Day,
  /// A weekly aggregate.
  Week,
  /// A monthly aggregate.
  Month,
  /// A quarterly aggregate.
  Quarter,
  /// A yearly aggregate.
  Year,
}

impl AsRef<str> for TimeSpan {
  fn as_ref(&self) -> &'static str {
    match *self {
      TimeSpan::Minute => "minute",
      TimeSpan::Hour => "hour",
      TimeSpan::Day => "day",
      TimeSpan::Week => "week",
      TimeSpan::Month => "month",
      TimeSpan::Quarter => "quarter",
      TimeSpan::Year => "year",
    }
  }
}


/// A GET request to be made to the
/// `/v2/aggs/ticker/<symbol>/range/1/<span>/<start>/<end>` endpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct AggregateReq {
  /// The ticker symbol to request aggregated data for.
  pub symbol: String,
  /// The aggregated time span.
  pub time_span: TimeSpan,
  /// The time span multiplier to use.
  pub multiplier: u8,
  /// The start date to request aggregates for.
  pub start_date: Date<Utc>,
  /// The end date to request aggregates for.
  ///
  /// Note that the reported the reported aggregates will include
  /// this date, i.e., the range is inclusive of this end date.
  pub end_date: Date<Utc>,
}


/// A ticker as returned by the
/// `/v2/aggs/ticker/<symbol>/range/1/<span>/<start>/<end>` endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Aggregate {
  /// The aggregate's timestamp.
  #[serde(rename = "t", deserialize_with = "datetime_from_timestamp")]
  pub timestamp: DateTime<Utc>,
  /// The trade volume during the aggregated time frame.
  ///
  /// This field's type is float because Polygon uses exponential format
  /// for the number, e.g., 3.5003466e+07.
  #[serde(rename = "v")]
  pub volume: f64,
  /// The open price.
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
}

type GetResponse = Response<Option<Vec<Aggregate>>>;

Endpoint! {
  /// The representation of a GET request to the
  /// `/v2/aggs/ticker/<symbol>/range/<multiplier>/<span>/<start>/<end>` endpoint.
  pub Get(AggregateReq),
  Ok => GetResponse, [
    /// The ticker information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(input: &Self::Input) -> Str {
    format!(
      "/v2/aggs/ticker/{sym}/range/{mult}/{span}/{start}/{end}",
      sym = input.symbol,
      mult = input.multiplier,
      span = input.time_span.as_ref(),
      start = input.start_date.format("%Y-%m-%d"),
      end = input.end_date.format("%Y-%m-%d"),
    ).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

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


  /// Make sure that we can deserialize an `Aggregate`.
  #[test]
  fn deserialize_aggregate() {
    let response = r#"{
  "v": 31315282,
  "o": 102.87,
  "c": 103.74,
  "h": 103.82,
  "l": 102.65,
  "t": 1549314000000,
  "n": 4
}"#;

    let aggregate = from_json::<Aggregate>(response).unwrap();
    assert_eq!(
      aggregate.timestamp,
      DateTime::parse_from_rfc3339("2019-02-04T16:00:00-05:00").unwrap(),
    );
    assert!(
      (aggregate.volume - 31_315_282f64).abs() <= EPSILON,
      "{}",
      aggregate.volume
    );
    assert_eq!(aggregate.open_price, Num::new(10287, 100));
    assert_eq!(aggregate.close_price, Num::new(10374, 100));
    assert_eq!(aggregate.high_price, Num::new(10382, 100));
    assert_eq!(aggregate.low_price, Num::new(10265, 100));
  }

  #[test]
  fn deserialize_response() {
    let response = r#"{
  "ticker": "AAPL",
  "status": "OK",
  "adjusted": true,
  "queryCount": 55,
  "resultsCount": 2,
  "results": [
    {
      "v": 31315282,
      "o": 102.87,
      "c": 103.74,
      "h": 103.82,
      "l": 102.65,
      "t": 1549314000000,
      "n": 4
    }
  ]
}"#;

    let mut aggregates = from_json::<GetResponse>(response)
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    assert_eq!(aggregates.len(), 1);

    let aggregate = aggregates.remove(0);
    assert!(
      (aggregate.volume - 31_315_282f64).abs() <= EPSILON,
      "{}",
      aggregate.volume
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_empty_aggregates() {
    let start = Utc.from_utc_date(&NaiveDate::from_str("2017-01-01").unwrap());
    let end = Utc.from_utc_date(&NaiveDate::from_str("2017-01-01").unwrap());

    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "VMW".into(),
      time_span: TimeSpan::Minute,
      multiplier: 5,
      start_date: start,
      end_date: end,
    };

    let result = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap_or_default();

    assert_eq!(result, Vec::new());
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_aapl_day_aggregates() {
    let start = Utc.from_utc_date(&NaiveDate::from_str("2021-11-01").unwrap());
    let end = Utc.from_utc_date(&NaiveDate::from_str("2021-11-30").unwrap());

    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "AAPL".into(),
      time_span: TimeSpan::Day,
      multiplier: 1,
      start_date: start,
      end_date: end,
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    // The number of trading days was inferred to be 21.
    assert_eq!(aggregates.len(), 21);
    assert_eq!(
      aggregates.first().unwrap().timestamp,
      // The offset here is only -04:00 because of daylight savings
      // time (which changed on November 6th).
      DateTime::parse_from_rfc3339("2021-11-01T00:00:00-04:00").unwrap()
    );
    assert_eq!(
      aggregates.last().unwrap().timestamp,
      DateTime::parse_from_rfc3339("2021-11-30T00:00:00-05:00").unwrap()
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_non_existent_aggregates() {
    let client = Client::from_env().unwrap();
    let today = Utc::today();
    let request = AggregateReq {
      symbol: "SPWR".into(),
      time_span: TimeSpan::Day,
      multiplier: 1,
      start_date: today + Duration::days(1),
      end_date: today + Duration::days(7),
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap();

    assert_eq!(aggregates, None);
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_spy_5min_aggregates() {
    let start = Utc.from_utc_date(&NaiveDate::from_str("2021-12-01").unwrap());
    let end = Utc.from_utc_date(&NaiveDate::from_str("2021-12-02").unwrap());

    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "SPY".into(),
      time_span: TimeSpan::Minute,
      multiplier: 5,
      start_date: start,
      end_date: end,
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    assert_eq!(aggregates.len(), 384);
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_xlk_hour_aggregates() {
    // Note that the Polygon API actually only supports retrieval of
    // data for the entire day. The granularity will still be an hour,
    // though.
    let start = Utc.from_utc_date(&NaiveDate::from_str("2021-12-06").unwrap());
    let end = Utc.from_utc_date(&NaiveDate::from_str("2021-12-06").unwrap());

    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "XLK".into(),
      time_span: TimeSpan::Hour,
      multiplier: 1,
      start_date: start,
      end_date: end,
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    // We expect 15 aggregates for the hours 4:00 to 19:00 (both inclusive).
    assert_eq!(aggregates.len(), 15);
    assert_eq!(
      aggregates[0].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T04:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[1].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T05:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[2].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T06:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[3].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T07:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[4].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T08:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[5].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T09:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[6].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T10:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[7].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T11:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[8].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T12:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[9].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T13:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[10].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T14:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[11].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T15:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[12].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T16:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[13].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T18:00:00-05:00").unwrap()
    );
    assert_eq!(
      aggregates[14].timestamp,
      DateTime::parse_from_rfc3339("2021-12-06T19:00:00-05:00").unwrap()
    );
  }

  /// Test that we can properly handle a response containing potentially
  /// "delayed" data.
  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn todays_data() {
    let client = Client::from_env().unwrap();
    let today = Utc::today();
    let request = AggregateReq {
      symbol: "SPY".into(),
      time_span: TimeSpan::Hour,
      multiplier: 1,
      start_date: today,
      end_date: today + Duration::days(1),
    };

    let _aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap();
  }
}
