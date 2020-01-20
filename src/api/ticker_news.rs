// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use serde::Deserialize;

use time_util::system_time_from_str;

use url::form_urlencoded::Serializer;

use crate::Str;


/// A GET request to be made to the /v1/meta/symbols/<ticker>/news
/// endpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct NewsReq {
  /// A ticker symbol.
  pub symbol: String,
  /// The page being retrieved.
  pub page: usize,
  /// The maximum number of results contained in one page.
  pub per_page: usize,
}

/// A ticker news item as returned by the /v1/meta/symbols/<ticker>/news
/// endpoint.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct News {
  /// The time the news item was published.
  #[serde(rename = "timestamp", deserialize_with = "system_time_from_str")]
  pub timestamp: SystemTime,
  /// The ticker symbols this news item relates to.
  #[serde(rename = "symbols")]
  pub symbols: Vec<String>,
  /// The title of the news item.
  #[serde(rename = "title")]
  pub title: String,
  /// The URL of the news item.
  #[serde(rename = "url")]
  pub url: String,
  /// The source of the news item.
  #[serde(rename = "source")]
  pub source: String,
  /// Keywords describing the news item.
  #[serde(rename = "keywords")]
  pub keywords: Vec<String>,
}


Endpoint! {
  /// The representation of a GET request to the GET
  /// /v1/meta/symbols/<ticker>/news endpoint.
  pub Get(NewsReq),
  Ok => Vec<News>, [
    /// The news information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(input: &Self::Input) -> Str {
    format!("/v1/meta/symbols/{}/news", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    let mut query = Serializer::new(String::new());
    query.append_pair("perpage", &input.per_page.to_string());
    query.append_pair("page", &input.page.to_string());

    Some(query.finish().into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use test_env_log::test;

  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn request_aapl_news() -> Result<(), Error> {
    let client = Client::from_env()?;
    let req = NewsReq {
      symbol: "AAPL".into(),
      per_page: 5,
      page: 1,
    };
    let news = client
      .issue::<Get>(req)
      .await
      .map_err(EndpointError::from)?;

    assert!(news.len() > 0, news);
    for item in news {
      assert!(item.symbols.contains(&"AAPL".to_string()), item);
    }
    Ok(())
  }
}
