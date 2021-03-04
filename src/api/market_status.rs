// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use serde::Deserialize;

use time_util::system_time_from_str;

use crate::Str;


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
  #[serde(rename = "serverTime", deserialize_with = "system_time_from_str")]
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

  use std::time::Duration;

  use test_env_log::test;

  use crate::Client;


  #[test(tokio::test)]
  async fn request_market_status() {
    const SECS_IN_HOUR: u64 = 60 * 60;

    let client = Client::from_env().unwrap();
    let market = client.issue::<Get>(()).await.unwrap();

    // We want to sanitize the current time being reported at least to a
    // certain degree. For that we assume that our local time is
    // somewhat synchronized to "real" time and are asserting that the
    // current time reported by Polygon is within one hour of our local
    // time (mainly to rule out wrong time zone handling).
    let now = SystemTime::now();
    let hour = Duration::from_secs(SECS_IN_HOUR);
    assert!(now > market.server_time - hour);
    assert!(now < market.server_time + hour);
  }
}
