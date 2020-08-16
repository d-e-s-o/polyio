// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::fmt::Debug;
use std::str::from_utf8;

use futures::Stream;

use http::request::Builder as HttpRequestBuilder;
use http::Request;
use http_endpoint::Endpoint;

use hyper::Body;
use hyper::body::to_bytes;
use hyper::Client as HttpClient;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;

use tracing::debug;
use tracing::instrument;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_futures::Instrument;

use serde_json::Error as JsonError;

use tungstenite::tungstenite::Error as WebSocketError;

use url::Url;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::error::RequestError;
use crate::events::Event;
use crate::events::stream;
use crate::events::Subscription;
use crate::events::Stock;

/// The query parameter used for communicating the API key to Polygon.
const API_KEY_PARAM: &str = "apiKey";


/// Normalize a list of subscriptions, removing duplicates and overlaps.
///
/// If a subscription applies to all stocks of a certain type (e.g.,
/// `Subscription::Trades(Stock::All)`) then more specific subscriptions
/// are removed (e.g., `Subscription::Trades(Stock::Symbol("SPY"))`).
fn normalize<S>(subscriptions: S) -> HashSet<Subscription>
where
  S: IntoIterator<Item = Subscription>,
{
  let mut subs = subscriptions.into_iter().collect::<HashSet<_>>();

  if subs.contains(&Subscription::SecondAggregates(Stock::All)) {
    subs.retain(|sub| match sub {
      Subscription::SecondAggregates(stock) => *stock == Stock::All,
      _ => true,
    })
  }

  if subs.contains(&Subscription::MinuteAggregates(Stock::All)) {
    subs.retain(|sub| match sub {
      Subscription::MinuteAggregates(stock) => *stock == Stock::All,
      _ => true,
    })
  }

  if subs.contains(&Subscription::Trades(Stock::All)) {
    subs.retain(|sub| match sub {
      Subscription::Trades(stock) => *stock == Stock::All,
      _ => true,
    })
  }

  if subs.contains(&Subscription::Quotes(Stock::All)) {
    subs.retain(|sub| match sub {
      Subscription::Quotes(stock) => *stock == Stock::All,
      _ => true,
    })
  }

  subs
}


/// A `Client` is the entity used by clients of this module for
/// interacting with the Polygon API.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
  client: HttpClient<HttpsConnector<HttpConnector>, Body>,
}

impl Client {
  /// Create a new `Client` using the given API information.
  pub fn new(api_info: ApiInfo) -> Self {
    let client = HttpClient::builder().build(HttpsConnector::new());
    Self { api_info, client }
  }

  /// Create a new `Client` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_info = ApiInfo::from_env()?;
    Ok(Self::new(api_info))
  }

  /// Build the URL for a request to the provided endpoint.
  fn url<E>(&self, input: &E::Input) -> Url
  where
    E: Endpoint,
  {
    let mut url = self.api_info.api_url.clone();
    url.set_path(&E::path(&input));
    url.set_query(E::query(&input).as_ref().map(AsRef::as_ref));
    url
      .query_pairs_mut()
      .append_pair(API_KEY_PARAM, &self.api_info.api_key);

    url
  }

  /// Create a `Request` to the endpoint.
  #[cfg(not(feature = "wasm"))]
  fn request<E>(&self, input: &E::Input) -> Result<Request<Body>, E::Error>
  where
    E: Endpoint,
  {
    let url = self.url::<E>(input);
    let request = HttpRequestBuilder::new()
      .method(E::method())
      .uri(url.as_str())
      .body(Body::from(E::body(input)?))?;

    Ok(request)
  }

  /// Create and issue a request and decode the response.
  #[instrument(level = "debug", skip(self, input))]
  #[allow(clippy::cognitive_complexity)]
  pub async fn issue<E>(&self, input: E::Input) -> Result<E::Output, RequestError<E::Error>>
  where
    E: Endpoint,
  {
    let req = self.request::<E>(&input).map_err(RequestError::Endpoint)?;
    let span = span!(
      Level::DEBUG,
      "request",
      method = display(&req.method()),
      url = display(&req.uri()),
    );

    async move {
      debug!("requesting");
      trace!(request = debug(&req));

      let result = self.client.request(req).await?;
      let status = result.status();
      debug!(status = debug(&status));
      trace!(response = debug(&result));

      let bytes = to_bytes(result.into_body()).await?;
      let body = bytes.as_ref();

      match from_utf8(body) {
        Ok(s) => trace!(body = display(&s)),
        Err(b) => trace!(body = display(&b)),
      }

      E::evaluate(status, body).map_err(RequestError::Endpoint)
    }
    .instrument(span)
    .await
  }

  /// Subscribe to the given stream in order to receive updates.
  pub async fn subscribe<S>(
    &self,
    subscriptions: S,
  ) -> Result<impl Stream<Item = Result<Result<Event, JsonError>, WebSocketError>>, Error>
  where
    S: IntoIterator<Item = Subscription>,
  {
    let subscriptions = normalize(subscriptions);
    self.subscribe_(subscriptions).await
  }

  /// Implementation of `subscribe` that creates a proper span.
  #[instrument(level = "debug", skip(self, subscriptions))]
  async fn subscribe_<S>(
    &self,
    subscriptions: S,
  ) -> Result<impl Stream<Item = Result<Result<Event, JsonError>, WebSocketError>>, Error>
  where
    S: IntoIterator<Item = Subscription> + Debug,
  {
    let mut url = self.api_info.stream_url.clone();
    url.set_scheme("wss").map_err(|()| {
      Error::Str(format!("unable to change URL scheme for {}: invalid URL?", url).into())
    })?;
    url.set_path("stocks");

    let api_info = ApiInfo {
      api_url: self.api_info.api_url.clone(),
      stream_url: url,
      api_key: self.api_info.api_key.clone(),
    };

    stream(api_info, subscriptions).await
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use maplit::hashset;

  use test_env_log::test;


  #[test]
  fn normalize_subscriptions() {
    let subscriptions = vec![
      Subscription::Quotes(Stock::Symbol("SPY".into())),
      Subscription::Trades(Stock::Symbol("MSFT".into())),
      Subscription::Quotes(Stock::All),
    ];
    let expected = hashset! {
      Subscription::Trades(Stock::Symbol("MSFT".into())),
      Subscription::Quotes(Stock::All),
    };
    assert_eq!(normalize(subscriptions), expected);

    let subscriptions = vec![
      Subscription::SecondAggregates(Stock::All),
      Subscription::SecondAggregates(Stock::Symbol("SPY".into())),
      Subscription::MinuteAggregates(Stock::Symbol("AAPL".into())),
      Subscription::MinuteAggregates(Stock::Symbol("VMW".into())),
      Subscription::MinuteAggregates(Stock::All),
    ];
    let expected = hashset! {
      Subscription::SecondAggregates(Stock::All),
      Subscription::MinuteAggregates(Stock::All),
    };
    assert_eq!(normalize(subscriptions), expected);

    let subscriptions = vec![
      Subscription::Trades(Stock::All),
      Subscription::Trades(Stock::Symbol("VMW".into())),
      Subscription::Trades(Stock::All),
    ];
    let expected = hashset! {
      Subscription::Trades(Stock::All),
    };
    assert_eq!(normalize(subscriptions), expected);
  }

  #[test(tokio::test)]
  async fn auth_failure() {
    let mut client = Client::from_env().unwrap();
    client.api_info.api_key = "not-a-valid-key".to_string();

    let result = client.subscribe(vec![]).await;
    match result {
      Err(Error::Str(err)) if err.starts_with("authentication not successful") => (),
      _ => panic!("unexpected result"),
    }
  }
}
