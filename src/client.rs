// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::Future;
use futures::stream::Stream;

use ratsio::error::RatsioError;

use crate::env::ApiInfo;
use crate::error::Error;
use crate::events::Event;
use crate::events::EventError;
use crate::events::subscribe;
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
  pub fn subscribe<S>(
    &self,
    subscriptions: S,
  ) -> Result<
    impl Future<Item = impl Stream<Item = Event, Error = EventError>, Error = RatsioError>,
    Error,
  >
  where
    S: IntoIterator<Item = Subscription>,
  {
    subscribe(&self.api_info.api_key, subscriptions)
  }
}
