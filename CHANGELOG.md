Unreleased
----------
- Migrated usage of `SystemTime` date times to `chrono` types
- Converted `api::Response` type into an `enum`
- Removed serialization support from event and aggregate data types
- Bumped minimum supported Rust version to `1.53`
- Bumped `http-endpoint` dependency to `0.5`
- Bumped `websocket-util` dependency to `0.10`


0.12.0
------
- Removed `api::tickers` and `api::ticker_news` module
- Switched from using `test-env-log` to `test-log`
- Bumped minimum supported Rust version to `1.51`
- Bumped `websocket-util` dependency to `0.9`
- Bumped `tokio-tungstenite` dependency to `0.16`


0.11.0
------
- Added `RateLimitExceeded` error variant to all endpoint errors
- Updated `num-decimal` to use version `0.4` of the `num-*` crates


0.10.0
------
- Renamed `api::market_status::Status` type to `Market`
- Introduced `Status` type representing market status
- Added support for handling delayed data
- Fixed parsing of market time to assume server-local time
- Bumped minimum supported Rust version to `1.46`
- Bumped `tokio-tungstenite` dependency to 0.14
- Bumped `websocket-util` dependency to `0.8`


0.9.0
-----
- Bumped minimum supported Rust version to `1.44`
- Replaced `async-tungstenite` dependency with `tokio-tungstenite`
- Bumped `hyper` dependency to `0.14`
- Bumped `hyper-tls` dependency to `0.5`
- Bumped `tokio` dependency to `1.0`
- Bumped `websocket-util` dependency to `0.7`


0.8.1
-----
- Added support for retrieving market status & time
- Excluded unnecessary files from being contained in release bundle


0.8.0
-----
- Reintroduced `Option` wrapper to aggregate response type
- Adjusted aggregate end date to be inclusive
- Fixed time stamp conversions to correctly honor daylight savings time
- Bumped minimum supported Rust version to `1.42`
- Bumped `time-util` dependency to `0.3`


0.7.0
-----
- Added support for `wasm32-unknown-unknown` target using Web APIs
- Return inner error as source in `RequestError::Endpoint` variant
- Introduced new `api::ResponseError` type
- Exported `api::Response` type
- Bumped `http-endpoint` dependency to `0.4`
- Bumped `websocket-util` dependency to `0.6`
- Bumped `async-tungstenite` dependency to `0.8`


0.6.0
-----
- Added `ApiInfo::new` constructor
- Removed `events::Trade::conditions` and `events::Quote::condition`
  members
- Use `thiserror` crate for defining error types
- Enabled CI pipeline comprising building and linting of the project
  - Added badge indicating pipeline status
- Bumped `http-endpoint` dependency to `0.3`
- Bumped `websocket-util` dependency to `0.5`
- Bumped `async-tungstenite` dependency to `0.5`


0.5.0
-----
- Introduced new public `events` module
- Removed `Hyper` variant from `Error` type
- Bumped `http-endpoint` dependency to `0.2`
  - Introduced new `RequestError` type
- Bumped `websocket-util` dependency to `0.4`


0.4.0
-----
- Added support for properly handling "disconnect" messages
- Adjusted stream functionality to stream a single event at a time
- Removed `Option` wrapper from aggregate response type
- Removed `Aggregate::accumulated_volume` and `Aggregate::average_price`
  members
- Decreased tracing verbosity by one level
- Bumped `num-decimal` dependency to `0.2`


0.3.0
-----
- Introduced `Client::new` constructor
- Assume Eastern time when deserializing streamed market data
- Serialize stock aggregates in millisecond format
- Removed `Aggregate::open_price_today` member


0.2.0
-----
- Bumped `time-util` dependency to `0.2`
- Bumped `websocket-util` dependency to `0.3`
- Bumped `async-tungstenite` dependency to `0.4`


0.1.2
-----
- Made `Aggregate`, `Quote`, and `Trade` types publicly available
- Added `Event::symbol` and `Subscription::stock` methods


0.1.1
-----
- Added support for retrieving stock aggregates
- Added support for retrieving ticker news items
- Made stream event types serializable
- Use `time-util` crate for time related operations


0.1.0
-----
- Initial release
