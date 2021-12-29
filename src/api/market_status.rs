// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::convert::TryFrom as _;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use chrono::DateTime;

use serde::de::Deserializer;
use serde::de::Error;
use serde::de::Unexpected;
use serde::Deserialize;

use crate::Str;


/// Deserialize a time stamp as a "naive local" `SystemTime`, i.e., one
/// which just drops any time zone offsets.
fn server_time<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  let time = String::deserialize(deserializer)?;
  DateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%z")
    .map_err(|_| Error::invalid_value(Unexpected::Str(&time), &"a date time string"))
    .and_then(|time| {
      u64::try_from(time.naive_local().timestamp())
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
        .map_err(|_| {
          Error::custom(format!(
            "seconds in {} could not be converted to unsigned value",
            time
          ))
        })
    })
}


/// The market status.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
pub enum Status {
  /// The market is currently open.
  #[serde(rename = "open")]
  Open,
  /// The market is currently closed.
  #[serde(rename = "closed")]
  Closed,
  /// Any other status that we have not accounted for.
  ///
  /// Note that having any such status should be considered a bug.
  #[serde(other)]
  Unknown,
}


/// An exchange as returned by the /v1/marketstatus/now endpoint.
///
/// Please note that not all fields available in a response are
/// represented here.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
pub struct Market {
  /// The status of the market as a whole.
  #[serde(rename = "market")]
  pub status: Status,
  /// The time the news item was published.
  #[serde(rename = "serverTime", deserialize_with = "server_time")]
  pub server_time: SystemTime,
}


Endpoint! {
  /// The representation of a GET request to the /v1/marketstatus/now
  /// endpoint.
  pub Get(()),
  Ok => Market, [
    /// The market status information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v1/marketstatus/now".into()
  }
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use super::*;

  use chrono::naive::NaiveTime;
  use chrono::offset::Utc;

  use test_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_market_status() {
    let client = Client::from_env().unwrap();
    let market = client.issue::<Get>(()).await.unwrap();
    let market_time = DateTime::<Utc>::from(market.server_time)
      .naive_local()
      .time();

    let open = NaiveTime::from_hms(9, 30, 0);
    let close = NaiveTime::from_hms(16, 00, 0);

    // We only make a statement when the market is open. It could be
    // closed for various reasons and sometimes trading days are just
    // short. We have never seen an overly long one, though.
    if market.status == Status::Open {
      assert!(market_time >= open);
      assert!(market_time < close);
    }
  }
}
