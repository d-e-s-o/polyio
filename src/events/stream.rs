// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use async_tls::TlsConnector;

use futures::Stream;

use log::debug;

use serde::Deserialize;
use serde_json::Error as JsonError;

use tungstenite::connect_async_with_tls_connector;
use tungstenite::tungstenite::Error as WebSocketError;

use websocket_util::stream as do_stream;

use crate::api_info::ApiInfo;
use crate::error::Error;
use crate::events::handshake::subscribe;
use crate::events::stock::Aggregate;
use crate::events::stock::Quote;
use crate::events::stock::Trade;
use crate::events::subscription::Subscription;


/// An enum representing the type of event we received from Polygon.
#[derive(Clone, Debug, Deserialize, PartialEq)]
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


/// A struct representing a number of events that occurred at the same
/// time.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Events(Vec<Event>);


/// Subscribe to and stream events from the Polygon service.
pub async fn stream<S>(
  api_info: ApiInfo,
  subscriptions: S,
) -> Result<impl Stream<Item = Result<Result<Events, JsonError>, WebSocketError>>, Error>
where
  S: IntoIterator<Item = Subscription>,
{
  let ApiInfo {
    stream_url: url,
    api_key,
  } = api_info;

  debug!("connecting to {}", &url);

  let connector = Some(TlsConnector::default());
  let (mut stream, _) = connect_async_with_tls_connector(url, connector).await?;

  subscribe(&mut stream, api_key, subscriptions).await?;

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

  use test_env_log::test;

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
      stream_url: Url::parse(&format!("ws://{}", addr.to_string())).unwrap(),
      api_key: API_KEY.to_string(),
    };

    stream(api_info, subscriptions).await
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

    let events = from_json::<Events>(&response).unwrap().0;
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

    let trade = stream.next().await.unwrap().unwrap().unwrap().0;
    assert_eq!(trade.len(), 1);
    assert_eq!(trade[0].to_trade().unwrap().symbol, "MSFT");

    let quote = stream.next().await.unwrap().unwrap().unwrap().0;
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
