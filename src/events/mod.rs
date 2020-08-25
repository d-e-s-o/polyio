// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(not(target_arch = "wasm32"))]
mod handshake;
#[cfg(not(target_arch = "wasm32"))]
mod stream;
mod subscription;

#[cfg(not(target_arch = "wasm32"))]
pub use stream::{
  stream,
  Aggregate,
  Event,
  Quote,
  Trade,
};
pub use subscription::Stock;
pub use subscription::Subscription;
