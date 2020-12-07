// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use chrono::offset::Utc;
use chrono::DateTime;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

use time_util::system_time_from_millis_in_new_york;
use time_util::system_time_to_millis_in_new_york;

use crate::api::response::Response;
use crate::Str;


/// Format a system time as a date.
fn format_date(time: &SystemTime) -> String {
  DateTime::<Utc>::from(*time)
    .date()
    .format("%Y-%m-%d")
    .to_string()
}


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
/// /v2/aggs/ticker/<symbol>/range/1/<span>/<start>/<end> endpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct AggregateReq {
  /// The ticker symbol to request aggregated data for.
  pub symbol: String,
  /// The aggregated time span.
  pub time_span: TimeSpan,
  /// The time span multiplier to use.
  pub multiplier: u8,
  /// The start time to request aggregates for.
  pub start_time: SystemTime,
  /// The end time to request aggregates for.
  ///
  /// Note that the reported the reported aggregates will include
  /// this time, i.e., the range is inclusive of this end date.
  pub end_time: SystemTime,
}


/// A ticker as returned by the
/// /v2/aggs/ticker/<symbol>/range/1/<span>/<start>/<end> endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Aggregate {
  /// The aggregate's timestamp.
  #[serde(
    rename = "t",
    deserialize_with = "system_time_from_millis_in_new_york",
    serialize_with = "system_time_to_millis_in_new_york",
  )]
  pub timestamp: SystemTime,
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
  /// /v2/aggs/ticker/<symbol>/range/<multiplier>/<span>/<start>/<end> endpoint.
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
      start = format_date(&input.start_time),
      end = format_date(&input.end_time),
    ).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::f64::EPSILON;
  use std::time::Duration;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  #[cfg(not(target_arch = "wasm32"))]
  use test_env_log::test;

  use time_util::parse_system_time_from_str;

  #[cfg(not(target_arch = "wasm32"))]
  use crate::Client;


  #[test]
  fn deserialize_serialize_aggregate() {
    let response = r#"{
  "v": 31315282,
  "o": 102.87,
  "c": 103.74,
  "h": 103.82,
  "l": 102.65,
  "t": 1549314000000,
  "n": 4
}"#;

    let aggregate = from_json::<Aggregate>(&response).unwrap();
    assert_eq!(
      aggregate.timestamp,
      parse_system_time_from_str("2019-02-04T16:00:00Z").unwrap(),
    );
    assert!(
      (aggregate.volume - 31_315_282f64).abs() <= EPSILON,
      aggregate.volume
    );
    assert_eq!(aggregate.open_price, Num::new(10287, 100));
    assert_eq!(aggregate.close_price, Num::new(10374, 100));
    assert_eq!(aggregate.high_price, Num::new(10382, 100));
    assert_eq!(aggregate.low_price, Num::new(10265, 100));

    let json = to_json(&aggregate).unwrap();
    let new = from_json::<Aggregate>(&json).unwrap();
    assert_eq!(new, aggregate);
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

    let mut aggregates = from_json::<GetResponse>(&response)
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    assert_eq!(aggregates.len(), 1);

    let aggregate = aggregates.remove(0);
    assert!(
      (aggregate.volume - 31_315_282f64).abs() <= EPSILON,
      aggregate.volume
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_empty_aggregates() {
    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "VMW".into(),
      time_span: TimeSpan::Minute,
      multiplier: 5,
      start_time: parse_system_time_from_str("2017-01-01T00:00:00Z").unwrap(),
      end_time: parse_system_time_from_str("2017-01-01T00:00:00Z").unwrap(),
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
    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "AAPL".into(),
      time_span: TimeSpan::Day,
      multiplier: 1,
      start_time: parse_system_time_from_str("2018-02-01T00:00:00Z").unwrap(),
      end_time: parse_system_time_from_str("2018-02-28T00:00:00Z").unwrap(),
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    // The number of trading days was inferred to be 19. There was
    // president's day on Feb 19th and all other days were regular work
    // days.
    assert_eq!(aggregates.len(), 19);
    assert_eq!(
      aggregates.first().unwrap().timestamp,
      parse_system_time_from_str("2018-02-01T00:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates.last().unwrap().timestamp,
      parse_system_time_from_str("2018-02-28T00:00:00Z").unwrap()
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_xlk_day_aggregates_daylight_savings() {
    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "XLK".into(),
      time_span: TimeSpan::Day,
      multiplier: 1,
      start_time: parse_system_time_from_str("2020-09-07T00:00:00Z").unwrap(),
      end_time: parse_system_time_from_str("2020-09-08T00:00:00Z").unwrap(),
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    assert_eq!(aggregates.len(), 1);
    assert_eq!(
      aggregates.first().unwrap().timestamp,
      parse_system_time_from_str("2020-09-08T00:00:00Z").unwrap()
    );
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_non_existent_aggregates() {
    let client = Client::from_env().unwrap();
    let today = SystemTime::now();
    let request = AggregateReq {
      symbol: "SPWR".into(),
      time_span: TimeSpan::Day,
      multiplier: 1,
      start_time: today + Duration::from_secs(24 * 60 * 60),
      end_time: today + 7 * Duration::from_secs(24 * 60 * 60),
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
    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "SPY".into(),
      time_span: TimeSpan::Minute,
      multiplier: 5,
      start_time: parse_system_time_from_str("2020-01-30T00:00:00Z").unwrap(),
      end_time: parse_system_time_from_str("2020-01-31T00:00:00Z").unwrap(),
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    assert_eq!(aggregates.len(), 383);
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test(tokio::test)]
  async fn request_xlk_hour_aggregates() {
    let client = Client::from_env().unwrap();
    let request = AggregateReq {
      symbol: "XLK".into(),
      time_span: TimeSpan::Hour,
      multiplier: 1,
      // Note that the Polygon API actually only supports retrieval of
      // data for the entire day. The granularity will still be an hour,
      // though.
      start_time: parse_system_time_from_str("2018-02-05T00:00:00Z").unwrap(),
      end_time: parse_system_time_from_str("2018-02-05T00:00:00Z").unwrap(),
    };

    let aggregates = client
      .issue::<Get>(request)
      .await
      .unwrap()
      .into_result()
      .unwrap()
      .unwrap();

    // We expect 13 aggregates for the hours 7:00 to 19:00 (both inclusive).
    assert_eq!(aggregates.len(), 13);
    assert_eq!(
      aggregates[0].timestamp,
      parse_system_time_from_str("2018-02-05T07:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[1].timestamp,
      parse_system_time_from_str("2018-02-05T08:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[2].timestamp,
      parse_system_time_from_str("2018-02-05T09:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[3].timestamp,
      parse_system_time_from_str("2018-02-05T10:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[4].timestamp,
      parse_system_time_from_str("2018-02-05T11:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[5].timestamp,
      parse_system_time_from_str("2018-02-05T12:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[6].timestamp,
      parse_system_time_from_str("2018-02-05T13:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[7].timestamp,
      parse_system_time_from_str("2018-02-05T14:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[8].timestamp,
      parse_system_time_from_str("2018-02-05T15:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[9].timestamp,
      parse_system_time_from_str("2018-02-05T16:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[10].timestamp,
      parse_system_time_from_str("2018-02-05T17:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[11].timestamp,
      parse_system_time_from_str("2018-02-05T18:00:00Z").unwrap()
    );
    assert_eq!(
      aggregates[12].timestamp,
      parse_system_time_from_str("2018-02-05T19:00:00Z").unwrap()
    );
  }
}
