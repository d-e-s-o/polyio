// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;
use std::io::Result as IoResult;

use futures::stream::Stream;

use crate::env::api_info;
use crate::error::Error;
use crate::events::Event;
use crate::events::EventError;
use crate::events::subscribe;
use crate::events::Subscription;


/// A `Client` is the entity used by clients of this module for
/// interacting with the Polygon API.
#[derive(Debug)]
pub struct Client {
  api_key: OsString,
}

impl Client {
  /// Create a new `Client` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_key = api_info()?;
    Ok(Self { api_key })
  }

  /// Subscribe to the given stream in order to receive updates.
  pub fn subscribe<'s, S>(
    &self,
    subscriptions: S,
  ) -> IoResult<impl Stream<Item = Event, Error = EventError>>
  where
    S: IntoIterator<Item = &'s Subscription>,
  {
    subscribe(&self.api_key, subscriptions)
  }
}
