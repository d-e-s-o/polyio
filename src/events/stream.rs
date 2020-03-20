// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use futures::Stream;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error as JsonError;

use time_util::system_time_from_millis_in_tz;
use time_util::system_time_to_millis_in_tz;
use time_util::EST;

use tracing::debug;
use tracing::trace;

use tungstenite::tokio::connect_async_with_tls_connector;
use tungstenite::tungstenite::Error as WebSocketError;

use websocket_util::stream as do_stream;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::events::handshake::handshake;
use crate::events::subscription::Subscription;


/// A data point for a trade.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Trade {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The exchange the trade occurred on.
  #[serde(rename = "x")]
  pub exchange: u64,
  /// The price.
  #[serde(rename = "p")]
  pub price: Num,
  /// The number of shares traded.
  #[serde(rename = "s")]
  pub quantity: u64,
  /// The trade conditions.
  #[serde(rename = "c")]
  pub conditions: Vec<u64>,
  /// The trade's timestamp (in UNIX milliseconds).
  #[serde(
    rename = "t",
    deserialize_with = "system_time_from_millis_in_tz::<EST, _>",
    serialize_with = "system_time_to_millis_in_tz::<EST, _>",
  )]
  pub timestamp: SystemTime,
}


/// A quote for a stock.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Quote {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The exchange where the stock is being asked for
  #[serde(rename = "bx")]
  pub bid_exchange: u64,
  /// The bid price.
  #[serde(rename = "bp")]
  pub bid_price: Num,
  /// The bid quantity
  #[serde(rename = "bs")]
  pub bid_quantity: u64,
  /// The exchange the trade occurred on.
  #[serde(rename = "ax")]
  pub ask_exchange: u64,
  /// The ask price.
  #[serde(rename = "ap")]
  pub ask_price: Num,
  /// The bid quantity
  #[serde(rename = "as")]
  pub ask_quantity: u64,
  /// The quote condition.
  #[serde(rename = "c")]
  pub condition: u64,
  /// The quote's timestamp (in UNIX milliseconds).
  #[serde(
    rename = "t",
    deserialize_with = "system_time_from_millis_in_tz::<EST, _>",
    serialize_with = "system_time_to_millis_in_tz::<EST, _>",
  )]
  pub timestamp: SystemTime,
}


/// An aggregate for a stock.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Aggregate {
  /// The stock's symbol.
  #[serde(rename = "sym")]
  pub symbol: String,
  /// The tick volume.
  #[serde(rename = "v")]
  pub volume: u64,
  /// Volume weighted average price.
  #[serde(rename = "vw")]
  pub volume_weighted_average_price: Num,
  /// The tick's open price.
  #[serde(rename = "o")]
  pub open_price: Num,
  /// The tick's close price.
  #[serde(rename = "c")]
  pub close_price: Num,
  /// The tick's high price.
  #[serde(rename = "h")]
  pub high_price: Num,
  /// The tick's low price.
  #[serde(rename = "l")]
  pub low_price: Num,
  /// The tick's start timestamp (in UNIX milliseconds).
  #[serde(
    rename = "s",
    deserialize_with = "system_time_from_millis_in_tz::<EST, _>",
    serialize_with = "system_time_to_millis_in_tz::<EST, _>",
  )]
  pub start_timestamp: SystemTime,
  /// The tick's end timestamp (in UNIX milliseconds).
  #[serde(
    rename = "e",
    deserialize_with = "system_time_from_millis_in_tz::<EST, _>",
    serialize_with = "system_time_to_millis_in_tz::<EST, _>",
  )]
  pub end_timestamp: SystemTime,
}


/// An enum representing the type of event we received from Polygon.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "ev")]
pub enum Event {
  /// A tick for a second aggregate for a stock.
  #[serde(rename = "A")]
  SecondAggregate(Aggregate),
  /// A tick for a minute aggregate for a stock.
  #[serde(rename = "AM")]
  MinuteAggregate(Aggregate),
  /// A tick for a trade of a stock.
  #[serde(rename = "T")]
  Trade(Trade),
  /// A tick for a quote for a stock.
  #[serde(rename = "Q")]
  Quote(Quote),
}

impl Event {
  /// Retrieve the event's symbol.
  pub fn symbol(&self) -> &str {
    match self {
      Event::SecondAggregate(aggregate) | Event::MinuteAggregate(aggregate) => &aggregate.symbol,
      Event::Trade(trade) => &trade.symbol,
      Event::Quote(quote) => &quote.symbol,
    }
  }

  #[cfg(test)]
  fn to_trade(&self) -> Option<&Trade> {
    match self {
      Event::Trade(trade) => Some(trade),
      _ => None,
    }
  }

  #[cfg(test)]
  fn to_quote(&self) -> Option<&Quote> {
    match self {
      Event::Quote(quote) => Some(quote),
      _ => None,
    }
  }
}


/// A type representing a number of events that occurred at the same
/// time.
pub type Events = Vec<Event>;


/// Subscribe to and stream events from the Polygon service.
pub async fn stream<S>(
  api_info: ApiInfo,
  subscriptions: S,
) -> Result<impl Stream<Item = Result<Result<Events, JsonError>, WebSocketError>>, Error>
where
  S: IntoIterator<Item = Subscription>,
{
  let ApiInfo {
    api_url: _,
    stream_url: url,
    api_key,
  } = api_info;

  debug!(message = "connecting", url = display(&url));

  let (mut stream, response) = connect_async_with_tls_connector(url, None).await?;
  debug!("connection successful");
  trace!(response = debug(&response));

  handshake(&mut stream, api_key, subscriptions).await?;
  debug!("subscription successful");

  let stream = do_stream(stream).await;
  Ok(stream)
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::future::Future;

  use futures::future::ready;
  use futures::SinkExt;
  use futures::StreamExt;
  use futures::TryStreamExt;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  use test_env_log::test;

  use time_util::parse_system_time_from_str;

  use tungstenite::tungstenite::Message;

  use url::Url;

  use websocket_util::test::mock_server;
  use websocket_util::test::WebSocketStream;

  use crate::events::subscription::Stock;

  const API_KEY: &str = "USER12345678";
  const CONNECTED_MSG: &str =
    { r#"[{"ev":"status","status":"connected","message":"Connected Successfully"}]"# };
  const AUTH_REQ: &str = r#"{"action":"auth","params":"USER12345678"}"#;
  const AUTH_RESP: &str = r#"[{"ev":"status","status":"auth_success","message":"authenticated"}]"#;
  const SUB_REQ: &str = r#"{"action":"subscribe","params":"T.MSFT,Q.*"}"#;
  const SUB_RESP: &str = {
    r#"[
      {"ev":"status","status":"success","message":"subscribed to: T.MSFT"},
      {"ev":"status","status":"success","message":"subscribed to: Q.*"}]
    "#
  };
  const MSFT_TRADE_MSG: &str = {
    r#"[{"ev":"T","sym":"MSFT","i":8310,"x":4,"p":156.9799,"s":3,"c":[37],"t":1577818283019,"z":3}]"#
  };
  const UFO_QUOTE_MSG: &str = {
    r#"[
      {"ev":"Q","sym":"UFO","c":1,"bx":8,"ax":12,"bp":26.4,"ap":26.47,"bs":1,"as":3,"t":1577818659363,"z":3},
      {"ev":"Q","sym":"UFO","c":1,"bx":8,"ax":12,"bp":26.4,"ap":26.47,"bs":1,"as":11,"t":1577818659365,"z":3}
    ]"#
  };


  async fn mock_stream<F, R, S>(
    f: F,
    subscriptions: S,
  ) -> Result<impl Stream<Item = Result<Result<Events, JsonError>, WebSocketError>>, Error>
  where
    F: FnOnce(WebSocketStream) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), WebSocketError>> + Send + Sync + 'static,
    S: IntoIterator<Item = Subscription>,
  {
    let addr = mock_server(f).await;
    let api_info = ApiInfo {
      api_url: Url::parse("http://example.com").unwrap(),
      stream_url: Url::parse(&format!("ws://{}", addr.to_string())).unwrap(),
      api_key: API_KEY.to_string(),
    };

    stream(api_info, subscriptions).await
  }

  #[test]
  fn deserialize_serialize_trade() {
    let response = r#"{
      "ev": "T",
      "sym": "SPY",
      "i": 436698869,
      "x": 19,
      "p": 293.67,
      "s": 100,
      "c": [],
      "t": 1583527402638,
      "z": 2
    }"#;
    let trade = from_json::<Trade>(&response).unwrap();
    assert_eq!(trade.symbol, "SPY");
    assert_eq!(trade.exchange, 19);
    assert_eq!(trade.price, Num::new(29367, 100));
    assert_eq!(trade.quantity, 100);
    assert_eq!(trade.conditions, Vec::<u64>::new());
    assert_eq!(
      trade.timestamp,
      parse_system_time_from_str("2020-03-06T15:43:22.638Z").unwrap()
    );

    let json = to_json(&trade).unwrap();
    let new = from_json::<Trade>(&json).unwrap();
    assert_eq!(new, trade);
  }

  #[test]
  fn deserialize_serialize_quote() {
    let response = r#"{
      "ev": "Q",
      "sym": "SPY",
      "c": 0,
      "bx": 12,
      "ax": 11,
      "bp": 294.31,
      "ap": 294.33,
      "bs": 1,
      "as": 2,
      "t": 1583527004684,
      "z": 2
    }"#;
    let quote = from_json::<Quote>(&response).unwrap();
    assert_eq!(quote.symbol, "SPY");
    assert_eq!(quote.bid_exchange, 12);
    assert_eq!(quote.bid_price, Num::new(29431, 100));
    assert_eq!(quote.bid_quantity, 1);
    assert_eq!(quote.ask_exchange, 11);
    assert_eq!(quote.ask_price, Num::new(29433, 100));
    assert_eq!(quote.ask_quantity, 2);
    assert_eq!(quote.condition, 0);
    assert_eq!(
      quote.timestamp,
      parse_system_time_from_str("2020-03-06T15:36:44.684Z").unwrap()
    );

    let json = to_json(&quote).unwrap();
    let new = from_json::<Quote>(&json).unwrap();
    assert_eq!(new, quote);
  }

  #[test]
  fn deserialize_serialize_aggregate() {
    let response = r#"{
      "ev": "A",
      "sym": "SPY",
      "v": 2287,
      "av": 163569633,
      "op": 298.71,
      "vw": 294.6301,
      "o": 293.79,
      "c": 293.68,
      "h": 293.8,
      "l": 293.68,
      "a": 293.7442,
      "s": 1583527401000,
      "e": 1583527402000
    }"#;

    let aggregate = from_json::<Aggregate>(&response).unwrap();
    assert_eq!(aggregate.symbol, "SPY");
    assert_eq!(aggregate.volume, 2287);
    assert_eq!(
      aggregate.volume_weighted_average_price,
      Num::new(2946301, 10000),
    );
    assert_eq!(aggregate.open_price, Num::new(29379, 100));
    assert_eq!(aggregate.close_price, Num::new(29368, 100));
    assert_eq!(aggregate.high_price, Num::new(2938, 10));
    assert_eq!(aggregate.low_price, Num::new(29368, 100));
    assert_eq!(
      aggregate.start_timestamp,
      parse_system_time_from_str("2020-03-06T15:43:21Z").unwrap()
    );
    assert_eq!(
      aggregate.end_timestamp,
      parse_system_time_from_str("2020-03-06T15:43:22Z").unwrap()
    );

    let json = to_json(&aggregate).unwrap();
    let new = from_json::<Aggregate>(&json).unwrap();
    assert_eq!(new, aggregate);
  }

  #[test]
  fn parse_event() {
    let response = r#"{
      "ev": "AM",
      "sym": "MSFT",
      "v": 10204,
      "av": 200304,
      "op": 114.04,
      "vw": 114.4040,
      "o": 114.11,
      "c": 114.14,
      "h": 114.19,
      "l": 114.09,
      "a": 114.1314,
      "s": 1536036818784,
      "e": 1536036818784
    }"#;

    let event = from_json::<Event>(&response).unwrap();
    match event {
      Event::MinuteAggregate(aggregate) => {
        assert_eq!(aggregate.symbol, "MSFT");
      },
      _ => panic!("unexpected event: {:?}", event),
    }
  }

  #[test]
  fn parse_events() {
    let response = r#"[
      {"ev":"Q","sym":"XLE","c":0,"bx":11,"ax":12,"bp":59.88,
       "ap":59.89,"bs":28,"as":67,"t":1577724127207,"z":2},
      {"ev":"Q","sym":"AAPL","c":0,"bx":11,"ax":12,"bp":59.88,
       "ap":59.89,"bs":28,"as":65,"t":1577724127207,"z":2}
    ]"#;

    let events = from_json::<Events>(&response).unwrap();
    assert_eq!(events.len(), 2);
    match &events[0] {
      Event::Quote(Quote { symbol, .. }) if symbol == "XLE" => (),
      e => panic!("unexpected event: {:?}", e),
    }
    match &events[1] {
      Event::Quote(Quote { symbol, .. }) if symbol == "AAPL" => (),
      e => panic!("unexpected event: {:?}", e),
    }
  }

  #[test(tokio::test)]
  async fn stream_msft() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      stream
        .send(Message::Text(CONNECTED_MSG.to_string()))
        .await?;

      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(SUB_REQ.to_string()),
      );
      stream.send(Message::Text(SUB_RESP.to_string())).await?;

      stream
        .send(Message::Text(MSFT_TRADE_MSG.to_string()))
        .await?;
      stream
        .send(Message::Text(UFO_QUOTE_MSG.to_string()))
        .await?;
      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let subscriptions = vec![
      Subscription::Trades(Stock::Symbol("MSFT".into())),
      Subscription::Quotes(Stock::All),
    ];
    let mut stream = Box::pin(mock_stream(test, subscriptions).await.unwrap());

    let trade = stream.next().await.unwrap().unwrap().unwrap();
    assert_eq!(trade.len(), 1);
    assert_eq!(trade[0].to_trade().unwrap().symbol, "MSFT");

    let quote = stream.next().await.unwrap().unwrap().unwrap();
    assert_eq!(quote.len(), 2);

    let quote0 = quote[0].to_quote().unwrap();
    assert_eq!(quote0.symbol, "UFO");
    assert_eq!(quote0.ask_quantity, 3);

    let quote1 = quote[1].to_quote().unwrap();
    assert_eq!(quote1.symbol, "UFO");
    assert_eq!(quote1.ask_quantity, 11);
  }

  #[test(tokio::test)]
  async fn interleaved_trade() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      stream
        .send(Message::Text(CONNECTED_MSG.to_string()))
        .await?;

      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(SUB_REQ.to_string()),
      );

      // We have seen cases where the subscription response is actually
      // preceded by an event we just subscribed to. Ugh. So simulate
      // such a case to make sure we can deal with such races.
      stream
        .send(Message::Text(MSFT_TRADE_MSG.to_string()))
        .await?;
      stream.send(Message::Text(SUB_RESP.to_string())).await?;

      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let subscriptions = vec![
      Subscription::Trades(Stock::Symbol("MSFT".into())),
      Subscription::Quotes(Stock::All),
    ];
    let _ = mock_stream(test, subscriptions)
      .await
      .unwrap()
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }
}
