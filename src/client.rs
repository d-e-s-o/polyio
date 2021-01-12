// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::fmt::Debug;

#[cfg(not(target_arch = "wasm32"))]
use futures::Stream;

use http_endpoint::Endpoint;

use tracing::debug;
use tracing::instrument;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_futures::Instrument;

#[cfg(not(target_arch = "wasm32"))]
use serde_json::Error as JsonError;

use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use websocket_util::tungstenite::Error as WebSocketError;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::error::RequestError;
use crate::events::Stock;
use crate::events::Subscription;
#[cfg(not(target_arch = "wasm32"))]
use crate::events::{
  stream,
  Event,
};

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


/// Build the URL for a request to the provided endpoint.
fn url<E>(api_info: &ApiInfo, input: &E::Input) -> Url
where
  E: Endpoint,
{
  let mut url = api_info.api_url.clone();
  url.set_path(&E::path(&input));
  url.set_query(E::query(&input).as_ref().map(AsRef::as_ref));
  url
    .query_pairs_mut()
    .append_pair(API_KEY_PARAM, &api_info.api_key);

  url
}


#[cfg(not(target_arch = "wasm32"))]
mod hype {
  use super::*;

  use std::str::from_utf8;

  use http::request::Builder as HttpRequestBuilder;
  use http::Request;

  use hyper::body::to_bytes;
  use hyper::client::HttpConnector;
  use hyper::Body;
  use hyper::Client as HttpClient;
  use hyper_tls::HttpsConnector;

  pub type Backend = HttpClient<HttpsConnector<HttpConnector>, Body>;

  pub fn new() -> Backend {
    HttpClient::builder().build(HttpsConnector::new())
  }

  /// Create a `Request` to the endpoint.
  fn request<E>(api_info: &ApiInfo, input: &E::Input) -> Result<Request<Body>, E::Error>
  where
    E: Endpoint,
  {
    let url = url::<E>(api_info, input);
    let request = HttpRequestBuilder::new()
      .method(E::method())
      .uri(url.as_str())
      .body(Body::from(E::body(input)?))?;

    Ok(request)
  }

  #[allow(clippy::cognitive_complexity)]
  pub async fn issue<E>(
    client: &Backend,
    api_info: &ApiInfo,
    input: E::Input,
  ) -> Result<E::Output, RequestError<E::Error>>
  where
    E: Endpoint,
  {
    let req = request::<E>(&api_info, &input).map_err(RequestError::Endpoint)?;
    let span = span!(
      Level::DEBUG,
      "request",
      method = display(&req.method()),
      url = display(&req.uri()),
    );

    async move {
      debug!("requesting");
      trace!(request = debug(&req));

      let result = client.request(req).await?;
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
}


#[cfg(target_arch = "wasm32")]
mod wasm {
  use super::*;

  use http::StatusCode;

  use js_sys::JSON::stringify;

  use wasm_bindgen::JsCast;
  use wasm_bindgen::JsValue;
  use wasm_bindgen_futures::JsFuture;

  use web_sys::window;
  use web_sys::Request;
  use web_sys::RequestInit;
  use web_sys::RequestMode;
  use web_sys::Response;
  use web_sys::Window;

  pub type Backend = Window;

  pub fn new() -> Backend {
    window().expect("no window found; not running inside a browser?")
  }

  /// Create a `Request` to the endpoint.
  fn request<E>(api_info: &ApiInfo, input: &E::Input) -> Result<Request, RequestError<E::Error>>
  where
    E: Endpoint,
  {
    let url = url::<E>(api_info, input);
    let body = E::body(input)
      .map_err(E::Error::from)
      .map_err(RequestError::Endpoint)?;

    let mut opts = RequestInit::new();
    opts.mode(RequestMode::Cors);
    opts.method(E::method().as_str());

    // And then check how *exactly* to retrieve the cause.
    if !body.is_empty() {
      let body = String::from_utf8(body.into_owned())?;
      opts.body(Some(&JsValue::from(body)));
    }

    let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
    Ok(request)
  }

  pub async fn issue<E>(
    client: &Backend,
    api_info: &ApiInfo,
    input: E::Input,
  ) -> Result<E::Output, RequestError<E::Error>>
  where
    E: Endpoint,
  {
    let req = request::<E>(api_info, &input)?;
    let span = span!(
      Level::DEBUG,
      "request",
      method = display(&req.method()),
      url = display(&req.url()),
    );

    async move {
      debug!("requesting");
      trace!(request = debug(&req));

      let response = JsFuture::from(client.fetch_with_request(&req)).await?;
      let response = response.dyn_into::<Response>()?;

      let status = response.status();
      debug!(status = debug(&status));
      trace!(response = debug(&response));

      let json = JsFuture::from(response.json().unwrap()).await?;
      let body = &String::from(&stringify(&json)?);
      trace!(body = display(&body));

      let status = StatusCode::from_u16(status)?;
      E::evaluate(status, body.as_bytes()).map_err(RequestError::Endpoint)
    }
    .instrument(span)
    .await
  }
}

#[cfg(not(target_arch = "wasm32"))]
use hype::*;
#[cfg(target_arch = "wasm32")]
use wasm::*;

/// A `Client` is the entity used by clients of this module for
/// interacting with the Polygon API.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
  client: Backend,
}

impl Client {
  /// Create a new `Client` using the given API information.
  pub fn new(api_info: ApiInfo) -> Self {
    let client = new();
    Self { api_info, client }
  }

  /// Create a new `Client` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_info = ApiInfo::from_env()?;
    Ok(Self::new(api_info))
  }

  /// Create and issue a request and decode the response.
  #[instrument(level = "debug", skip(self, input))]
  pub async fn issue<E>(&self, input: E::Input) -> Result<E::Output, RequestError<E::Error>>
  where
    E: Endpoint,
  {
    issue::<E>(&self.client, &self.api_info, input).await
  }

  /// Subscribe to the given stream in order to receive updates.
  #[cfg(not(target_arch = "wasm32"))]
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
  #[cfg(not(target_arch = "wasm32"))]
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

  #[cfg(not(target_arch = "wasm32"))]
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

  #[cfg(not(target_arch = "wasm32"))]
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
