// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use futures::future::Either;
use futures::future::err;
use futures::future::ok;
use futures::future::done;
use futures::Future;
use futures::stream::empty;
use futures::Stream;

use log::debug;

use ratsio::error::RatsioError;
use ratsio::nats_client::NatsClient;
use ratsio::nats_client::NatsClientOptions;
use ratsio::ops::Message;
use ratsio::ops::Subscribe;

use serde::Deserialize;
use serde_json::Error as JsonError;
use serde_json::from_slice as from_json;

use crate::error::Error;
use crate::stock::Aggregate;
use crate::stock::Quote;
use crate::stock::Trade;
use crate::Str;


const POLYGON_CLUSTER: [&str; 3] = [
  "nats1.polygon.io:31101",
  "nats2.polygon.io:31102",
  "nats3.polygon.io:31103",
];

fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
  write!(fmt, "{}", err)?;
  if let Some(src) = err.source() {
    write!(fmt, ": ")?;
    fmt_err(src, fmt)?;
  }
  Ok(())
}


#[derive(Debug)]
pub enum EventError {
  Json(JsonError),
  Ratsio(RatsioError),
  Str(Str),
}

impl Display for EventError {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      EventError::Json(err) => fmt_err(err, fmt),
      EventError::Ratsio(err) => write!(fmt, "{}", err),
      EventError::Str(err) => write!(fmt, "{}", err),
    }
  }
}

impl StdError for EventError {}


/// Possible subscriptions for a stock.
#[derive(Clone, Debug, PartialEq)]
pub enum Stock {
  /// Subscribe to the stock with the given symbol.
  Symbol(Str),
  /// Subscribe to an event type for all available stocks.
  All,
}

impl Display for Stock {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Stock::Symbol(symbol) => write!(fmt, "{}", symbol),
      Stock::All => write!(fmt, "*"),
    }
  }
}


/// An enum describing a subscription.
#[derive(Clone, Debug, PartialEq)]
pub enum Subscription {
  /// A type representing second aggregates for the given stock.
  SecondAggregates(Stock),
  /// A type representing minute aggregates for the given stock.
  MinuteAggregates(Stock),
  /// A type representing trades for the given stock.
  Trades(Stock),
  /// A type representing quotes for the given stock.
  Quotes(Stock),
}

impl Display for Subscription {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Subscription::SecondAggregates(stock) => write!(fmt, "A.{}", stock.to_string()),
      Subscription::MinuteAggregates(stock) => write!(fmt, "AM.{}", stock.to_string()),
      Subscription::Trades(stock) => write!(fmt, "T.{}", stock.to_string()),
      Subscription::Quotes(stock) => write!(fmt, "Q.{}", stock.to_string()),
    }
  }
}


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
}


fn do_subscribe<S>(
  client: &NatsClient,
  subscriptions: S,
) -> Result<impl Stream<Item = Message, Error = RatsioError>, RatsioError>
where
  S: IntoIterator<Item = Subscription>,
{
  let stream = empty();
  let mut iter = subscriptions.into_iter();

  // TODO: Right now we only support up to four subscriptions.
  //       Unfortunately ratsio does not allow us to simply
  //       subscribe to multiple sources exposed through one stream
  //       and so we create a new stream for each subscription.
  //       Because we want to expose all of them as a single stream
  //       (of a single opaque type) we have to add each
  //       subscription on demand.
  let stream = if let Some(subscription) = iter.next() {
    let other = client
      .subscribe({
        Subscribe::builder()
          .subject(subscription.to_string().into())
          .build()?
      })
      .flatten_stream();

    Either::A(stream.select(other))
  } else {
    Either::B(stream)
  };

  let stream = if let Some(subscription) = iter.next() {
    let other = client
      .subscribe({
        Subscribe::builder()
          .subject(subscription.to_string().into())
          .build()?
      })
      .flatten_stream();

    Either::A(stream.select(other))
  } else {
    Either::B(stream)
  };

  let stream = if let Some(subscription) = iter.next() {
    let other = client
      .subscribe({
        Subscribe::builder()
          .subject(subscription.to_string().into())
          .build()?
      })
      .flatten_stream();

    Either::A(stream.select(other))
  } else {
    Either::B(stream)
  };

  let stream = if let Some(subscription) = iter.next() {
    let other = client
      .subscribe({
        Subscribe::builder()
          .subject(subscription.to_string().into())
          .build()?
      })
      .flatten_stream();

    Either::A(stream.select(other))
  } else {
    Either::B(stream)
  };

  assert_eq!(iter.next(), None, "only up to four subscriptions are supported currently");

  Ok(stream)
}


fn subscribe_to_cluster<'c, C, S>(
  api_key: &str,
  cluster: C,
  subscriptions: S,
) -> Result<
  impl Future<Item = impl Stream<Item = Event, Error = EventError>, Error = RatsioError>,
  Error,
>
where
  C: Into<Vec<String>>,
  S: IntoIterator<Item = Subscription>,
{
  let options = NatsClientOptions::builder()
    .echo(true)
    .verbose(true)
    .cluster_uris(cluster.into())
    .auth_token(api_key)
    .build()
    .map_err(|err| Error::Str(err.into()))?;

  debug!("NATS client options: {:?}", options);

  let stream = NatsClient::from_options(options)
    .and_then(|client| NatsClient::connect(&client))
    // TODO: Use client.add_reconnect_handler?
    .and_then(|client| {
      let stream = do_subscribe(&client, subscriptions)
        // TODO: Should not unwrap!
        .unwrap()
        .map_err(EventError::Ratsio)
        .and_then(|msg| {
          debug!("Received message: {:?}", &msg);
          if msg.subject.starts_with("A.") {
            done(from_json::<Aggregate>(&msg.payload)
              .map(Event::SecondAggregate)
              .map_err(EventError::Json))
          } else if msg.subject.starts_with("AM.") {
            done(from_json::<Aggregate>(&msg.payload)
              .map(Event::MinuteAggregate)
              .map_err(EventError::Json))
          } else if msg.subject.starts_with("T.") {
            done(from_json::<Trade>(&msg.payload)
              .map(Event::Trade)
              .map_err(EventError::Json))
          } else if msg.subject.starts_with("Q.") {
            done(from_json::<Quote>(&msg.payload)
              .map(Event::Quote)
              .map_err(EventError::Json))
          } else {
            err(EventError::Str(format!("received unexpected subject: {}", msg.subject).into()))
          }
        });

      ok(stream)
    });

  Ok(stream)
}

/// Subscribe to and stream events from the Polygon service.
pub fn subscribe<S>(
  api_key: &str,
  subscriptions: S,
) -> Result<
  impl Future<Item = impl Stream<Item = Event, Error = EventError>, Error = RatsioError>,
  Error,
>
where
  S: IntoIterator<Item = Subscription>,
{
  let cluster = POLYGON_CLUSTER.iter().map(|x| x.to_string()).collect::<Vec<_>>();
  subscribe_to_cluster(api_key, cluster, subscriptions)
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::collections::BTreeMap;
  use std::net::SocketAddr;

  use futures::future::ok;
  use futures::Future;
  use futures::Sink;
  use futures::Stream;
  use futures::sync::mpsc;

  use num_decimal::Num;

  use ratsio::codec::OpCodec;
  use ratsio::error::RatsioError;
  use ratsio::ops::Message;
  use ratsio::ops::Op;
  use ratsio::ops::ServerInfo;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use tokio::runtime::Runtime;
  use tokio::runtime::TaskExecutor;
  use tokio_codec::Decoder;
  use tokio_tcp::TcpListener;


  fn serve_nats(
    executor: TaskExecutor,
    events: BTreeMap<&'static str, Vec<&'static str>>,
  ) -> Result<SocketAddr, RatsioError>
  {
    let address = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(&address).unwrap();
    let address = listener.local_addr().unwrap();
    let executor2 = executor.clone();

    let future = listener
      .incoming()
      .map(move |socket| OpCodec::default().framed(socket))
      .from_err()
      .and_then(|socket| {
        socket.send(Op::INFO(ServerInfo {
          server_id: "events-test".to_string(),
          version: "0.1".to_string(),
          go: "".to_string(),
          host: "localhost".to_string(),
          port: 0,
          max_payload: usize::max_value(),
          proto: 0,
          client_id: 0,
          auth_required: true,
          tls_required: false,
          tls_verify: false,
          connect_urls: Vec::new(),
        }))
      })
      .and_then(|socket| socket.send(Op::PING))
      .and_then(move |socket| {
        let (sink, stream) = socket.split();
        let (send, recv) = mpsc::unbounded();
        let recv = recv.map_err(|_| RatsioError::InnerBrokenChain);
        let executor = executor.clone();
        executor.spawn({
          sink
            .send_all(recv)
            .map(|_| ())
            .map_err(|err| assert!(false, "{:?}", err))
        });
        let mut events = events.clone();

        stream.for_each(move |op| {
          debug!("Got OP from client {:#?}", op);
          match op {
            Op::PONG => {
              debug!("Got PONG from client");
            }
            Op::PING => {
              let _ = send.unbounded_send(Op::PONG);
            }
            Op::SUB(cmd) => {
              // Once we got a subscription request for a subject we
              // remove that subject from the map of available ones.
              match events.remove(cmd.subject.as_str()) {
                Some(events) => {
                  executor.spawn({
                    for event in &events {
                      let message = Message {
                        subject: cmd.subject.clone(),
                        sid: cmd.sid.clone(),
                        reply_to: None,
                        payload: event.as_bytes().to_vec(),
                      };
                      let _ = send.unbounded_send(Op::MSG(message));
                    };
                    ok(())
                  })
                },
                None => panic!("encountered subscription for non-existent subject"),
              }
            }
            _ => {}
          }

          ok(())
        })
      })
      .into_future()
      .map(|_| ())
      .map_err(|_| ());

    executor2.spawn(future);
    Ok(address)
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
  fn receive_trades() {
    let mut runtime = Runtime::new().unwrap();
    let executor = runtime.executor();

    let subscriptions = vec![
      Subscription::Trades(Stock::Symbol("AAPL".into())),
    ];
    let mut events = BTreeMap::new();
    events.insert(
      "T.AAPL",
      vec![
        r#"{"ev":"T","sym":"AAPL","x":4,"p":194.949,"s":5,"c":[37],"t":1560181868990}"#,
        r#"{"ev":"T","sym":"AAPL","x":12,"p":194.97,"s":22,"c":[37],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":12,"p":194.94,"s":6,"c":[37],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":12,"p":194.96,"s":60,"c":[37],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":12,"p":194.93,"s":29,"c":[37],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":12,"p":194.91,"s":71,"c":[14,37,41],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":11,"p":194.95,"s":100,"c":[14,41],"t":1560181869197}"#,
        r#"{"ev":"T","sym":"AAPL","x":4,"p":194.955,"s":100,"c":[],"t":1560181869209}"#,
      ],
    );

    let address = serve_nats(executor, events).unwrap().to_string();
    let cluster = vec![address];
    let future = subscribe_to_cluster("xxxx", cluster, subscriptions.into_iter()).unwrap()
      .map_err(EventError::Ratsio)
      .flatten_stream()
      .skip(1)
      .take(5)
      .collect();

    let events = runtime.block_on(future).unwrap();
    assert_eq!(events.len(), 5);
    let trade = events[0].to_trade().unwrap();
    assert_eq!(&trade.symbol, "AAPL");
    assert_eq!(trade.price, Num::new(19497, 100));
    assert_eq!(trade.quantity, 22);
  }
}
