// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Debug;
use std::str::from_utf8;

use futures::Stream;

use http_endpoint::Endpoint;

use hyper::Body;
use hyper::body::to_bytes;
use hyper::Client as HttpClient;
use hyper::client::HttpConnector;
use hyper::http::request::Builder as HttpRequestBuilder;
use hyper::Request;
use hyper_tls::HttpsConnector;

use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::instrument;

use serde_json::Error as JsonError;

use tungstenite::tungstenite::Error as WebSocketError;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::events::Events;
use crate::events::stream;
use crate::events::Subscription;

/// The query parameter used for communicating the API key to Polygon.
const API_KEY_PARAM: &str = "apiKey";


/// A `Client` is the entity used by clients of this module for
/// interacting with the Polygon API.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
  client: HttpClient<HttpsConnector<HttpConnector>, Body>,
}

impl Client {
  /// Create a new `Client` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_info = ApiInfo::from_env()?;
    let client = HttpClient::builder().build(HttpsConnector::new());

    Ok(Self { api_info, client })
  }

  /// Create a `Request` to the endpoint.
  fn request<E>(&self, input: &E::Input) -> Result<Request<Body>, E::Error>
  where
    E: Endpoint,
  {
    let mut url = self.api_info.api_url.clone();
    url.set_path(&E::path(&input));
    url.set_query(E::query(&input).as_ref().map(AsRef::as_ref));
    url
      .query_pairs_mut()
      .append_pair(API_KEY_PARAM, &self.api_info.api_key);

    let request = HttpRequestBuilder::new()
      .method(E::method())
      .uri(url.as_str())
      .body(E::body(input)?)?;

    Ok(request)
  }

  /// Create and issue a request and decode the response.
  #[instrument(level = "info", skip(self, input))]
  pub async fn issue<E>(&self, input: E::Input) -> Result<E::Output, E::Error>
  where
    E: Endpoint,
  {
    let req = self.request::<E>(&input)?;
    let span = info_span!(
      "request",
      method = display(&req.method()),
      url = display(&req.uri()),
    );
    let _guard = span.enter();
    info!("requesting");
    debug!(request = debug(&req));

    let result = self.client.request(req).await?;
    let status = result.status();
    info!(status = debug(&status));
    debug!(response = debug(&result));

    let bytes = to_bytes(result.into_body()).await?;
    let body = bytes.as_ref();

    match from_utf8(body) {
      Ok(s) => debug!(body = display(&s)),
      Err(b) => debug!(body = display(&b)),
    }

    E::evaluate(status, body)
  }

  /// Subscribe to the given stream in order to receive updates.
  // TODO: Debug printing an iterator can yield some pretty nasty
  //       looking results. We may want to collect into a `Vec` or so to
  //       make the result easier digestible.
  #[instrument(level = "info", skip(self))]
  pub async fn subscribe<S>(
    &self,
    subscriptions: S,
  ) -> Result<impl Stream<Item = Result<Result<Events, JsonError>, WebSocketError>>, Error>
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

  use test_env_log::test;


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
