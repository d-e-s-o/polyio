// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

/// A response type used in certain API calls.
pub use response::Response;
/// An error type for responses indicating failures.
pub use response::ResponseError;

mod response;

/// Definitions surrounding aggregate prices of stocks.
pub mod aggregates;
/// Definitions surrounding quote prices of stocks.
pub mod quotes;
/// Definitions pertaining the available exchanges.
pub mod exchanges;
/// Definitions pertaining the available locales.
pub mod locales;
/// Definitions for retrieving the current market status.
pub mod market_status;
/// Definitions pertaining the available markets.
pub mod markets;
/// Definitions pertaining a ticker.
pub mod ticker;
/// Definitions for retrieving the available ticker types.
pub mod ticker_types;

