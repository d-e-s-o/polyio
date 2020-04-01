// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

#![type_length_limit = "536870912"]

#[macro_use]
extern crate http_endpoint;

#[macro_use]
mod endpoint;

/// A module comprising the functionality backing interactions with the
/// API.
pub mod api;

mod api_info;
mod client;
mod error;
mod events;

use std::borrow::Cow;

pub use api_info::ApiInfo;
pub use client::Client;
pub use error::Error;
pub use error::RequestError;
pub use events::Aggregate;
pub use events::Event;
pub use events::Quote;
pub use events::Stock;
pub use events::Subscription;
pub use events::Trade;

type Str = Cow<'static, str>;
