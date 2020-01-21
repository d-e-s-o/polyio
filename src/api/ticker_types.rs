// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::api::response::Response;
use crate::Str;


/// A struct representing the ticker types.
///
/// Please note that not all fields available in a request are
/// represented here.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TickerTypes {
  /// A mapping from ticker types to descriptions.
  #[serde(rename = "types")]
  pub types: BTreeMap<String, String>,
  /// A mapping from index types to descriptions.
  #[serde(rename = "indexTypes")]
  pub index_types: BTreeMap<String, String>,
}

Endpoint! {
  /// The representation of a GET request to the /v2/reference/types endpoint.
  pub Get(()),
  Ok => Response<TickerTypes>, [
    /// The ticker types were retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/reference/types".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::Client;

  #[test(tokio::test)]
  async fn request_ticker_types() {
    let client = Client::from_env().unwrap();
    let types = client.issue::<Get>(()).await.unwrap();

    println!("{:?}", types);
  }
}
