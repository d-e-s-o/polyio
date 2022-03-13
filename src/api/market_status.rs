// Copyright (C) 2020-2022 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::de::Deserializer;
use serde::de::Error;
use serde::de::Unexpected;
use serde::Deserialize;

use crate::Str;


/// Deserialize a date time from a string.
fn datetime_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
  D: Deserializer<'de>,
{
  let time = String::deserialize(deserializer)?;
  DateTime::parse_from_rfc3339(&time)
    .map(|datetime| datetime.with_timezone(&Utc))
    .map_err(|_| Error::invalid_value(Unexpected::Str(&time), &"a date time string"))
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


/// The market status as returned by the `/v1/marketstatus/now`
/// endpoint.
///
/// Please note that not all fields available in a response are
/// represented here.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
pub struct Market {
  /// The status of the market as a whole.
  #[serde(rename = "market")]
  pub status: Status,
  /// The current server time.
  #[serde(rename = "serverTime", deserialize_with = "datetime_from_str")]
  pub server_time: DateTime<Utc>,
}


Endpoint! {
  /// The representation of a GET request to the `/v1/marketstatus/now`
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

  use test_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_market_status() {
    let client = Client::from_env().unwrap();
    let market = client.issue::<Get>(()).await.unwrap();
    let market_time = market.server_time.naive_local().time();

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
