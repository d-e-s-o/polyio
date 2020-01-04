// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

mod api_info;
mod client;
mod error;
mod events;

use std::borrow::Cow;

pub use client::Client;
pub use error::Error;
pub use events::Event;
pub use events::Events;
pub use events::Stock;
pub use events::Subscription;

type Str = Cow<'static, str>;
