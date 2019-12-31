// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::Stream;

use serde_json::Error as JsonError;

use tungstenite::tungstenite::Error as WebSocketError;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::events::Events;
use crate::events::stream;
use crate::events::Subscription;


/// A `Client` is the entity used by clients of this module for
/// interacting with the Polygon API.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
}

impl Client {
  /// Create a new `Client` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_info = ApiInfo::from_env()?;

    Ok(Self { api_info })
  }

  /// Subscribe to the given stream in order to receive updates.
  pub async fn subscribe<S>(
    &self,
    subscriptions: S,
  ) -> Result<impl Stream<Item = Result<Result<Events, JsonError>, WebSocketError>>, Error>
  where
    S: IntoIterator<Item = Subscription>,
  {
    let mut url = self.api_info.stream_url.clone();
    url.set_scheme("wss").map_err(|()| {
      Error::Str(format!("unable to change URL scheme for {}: invalid URL?", url).into())
    })?;
    url.set_path("stocks");

    let api_info = ApiInfo {
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
