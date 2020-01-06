// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;

use crate::Str;


/// An exchange as returned by the /v1/meta/exchanges endpoint.
///
/// Please note that not all fields available in a request are
/// represented here.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Exchange {
  /// Exchange ID.
  #[serde(rename = "id")]
  pub id: usize,
  /// The type of exchange.
  #[serde(rename = "type")]
  pub type_: String,
  /// The type of market data the exchange provides.
  #[serde(rename = "market")]
  pub market: String,
  /// The exchange's name.
  #[serde(rename = "name")]
  pub name: String,
  /// The exchange's code.
  ///
  /// This field is seemingly only set for exchanges of type `Equities`.
  pub code: Option<String>,
}


Endpoint! {
  /// The representation of a GET request to the /v1/meta/exchanges endpoint.
  pub Get(()),
  Ok => Vec<Exchange>, [
    /// The exchanges information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v1/meta/exchanges".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_exchanges() {
    let response = r#"[
  {
    "id": 1,
    "type": "exchange",
    "market": "equities",
    "mic": "XASE",
    "name": "NYSE American (AMEX)",
    "tape": "A"
  },
  {
    "id": 2,
    "type": "exchange",
    "market": "equities",
    "mic": "XBOS",
    "name": "NASDAQ OMX BX",
    "tape": "B"
  },
  {
    "id": 15,
    "type": "exchange",
    "market": "equities",
    "mic": "IEXG",
    "name": "IEX",
    "tape": "V"
  },
  {
    "id": 16,
    "type": "TRF",
    "market": "equities",
    "mic": "XCBO",
    "name": "Chicago Board Options Exchange",
    "tape": "W"
  }
]"#;

    let exchgs = from_json::<Vec<Exchange>>(&response).unwrap();
    assert_eq!(exchgs.len(), 4);
    assert_eq!(exchgs[0].id, 1);
    assert_eq!(exchgs[0].type_, "exchange");
    assert_eq!(exchgs[0].market, "equities");
    assert_eq!(exchgs[0].name, "NYSE American (AMEX)");
    assert_eq!(exchgs[1].id, 2);
    assert_eq!(exchgs[2].id, 15);
    assert_eq!(exchgs[3].id, 16);
  }

  #[test(tokio::test)]
  async fn request_exchanges() -> Result<(), Error> {
    let client = Client::from_env()?;
    let exchgs = client.issue::<Get>(()).await.map_err(EndpointError::from)?;

    assert!(exchgs.len() > 0);

    // We are in trouble if NYE cannot be found.
    let nye = exchgs
      .iter()
      .find(|exchg| exchg.code.as_deref() == Some("NYE"))
      .unwrap();
    assert_eq!(nye.type_, "exchange");
    assert_eq!(nye.market, "equities");
    Ok(())
  }
}
