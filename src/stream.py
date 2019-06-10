# Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
# SPDX-License-Identifier: GPL-3.0-or-later

from argparse import (
  ArgumentParser,
  ArgumentTypeError,
)
from asyncio import (
  get_event_loop,
  sleep,
)
from os import (
  environ,
)
from signal import (
  SIGINT,
  SIGTERM,
)
from sys import (
  argv,
  stderr,
  stdout,
)
from textwrap import (
  fill,
)
from urllib.parse import (
  urlsplit,
  urlunsplit,
)

POLYGON_API_KEY = "POLYGON_API_KEY"
DESCRIPTION = fill(f"""\
Stream stock data from the Polygon service at polygon.io.

The {POLYGON_API_KEY} environment variable is expected to be set to the key
to use to authenticate with the API. The program requires the 'nats' Python
package which can be installed, for example, through pip:
  $ pip install asyncio-nats-client
""")


async def received_message(msg):
  """Process a received message by simply emitting it to stdout."""
  if msg.subject.startswith("A."):
    type_ = b"A"
  elif msg.subject.startswith("AM."):
    type_ = b"AM"
  elif msg.subject.startswith("Q."):
    type_ = b"Q"
  elif msg.subject.startswith("T."):
    type_ = b"T"

  # We munge with the JSON object to be able to identify it on the
  # receiving side. We do so according to how the WebSockets format is
  # documented to be laying out its data such that the client side can
  # stay the same once we make the switch over.
  # Also we use stdout.buffer because it allows us to work with bytes
  # directly, instead of having to convert to string first. That's also
  # the reason we shy away from the json package.
  # TODO: We may want to push out this synchronous write into an executor,
  #       e.g., via loop.run_in_executor, to not block the event loop.
  # TODO: Also, preferably we would want to print stuff in badges. It is not
  #       clear how to accomplish that, though.
  msg = msg.data.replace(b"{", b"""{"ev":"%s",""" % type_, 1)
  stdout.buffer.write(msg + b"\n")
  stdout.buffer.flush()


async def closed(loop):
  """A callback invoked when the NATS connection is closed."""
  print("Closed connection to server.", file=stderr)
  await sleep(0.1, loop=loop)
  loop.stop()


def add_auth_info(url, api_key):
  """Inject authentication information in the form of an API key into the given URL."""
  parts = urlsplit(url)
  if len(parts.scheme) == 0:
    parts = urlsplit("nats://%s" % url)

  if parts.username is not None or parts.password is not None:
    raise RuntimeError("server address %s already contains authentication information", url)

  scheme, netloc, path, params, fragment = parts
  netloc = "%s@%s" % (api_key, netloc)
  parts = (scheme, netloc, path, params, fragment)
  return urlunsplit(parts)


async def run(servers, api_key, events, loop):
  """Connect to Polygon, subscribe to a set of events, and stream."""
  # We import this non-standard package only locally to ensure that the program
  # will at least print a reasonable description even if the package is not
  # installed.
  from nats.aio.client import Client as NATS
  nats = NATS()

  options = {
    "servers": list(map(lambda server: add_auth_info(server, api_key), servers)),
    "io_loop": loop,
    "closed_cb": lambda: closed(loop),
  }

  await nats.connect(**options)
  print("Connected to Polygon", file=stderr)

  def handle_signal():
    if nats.is_closed:
      return
    print("Disconnecting...", file=stderr)
    loop.create_task(nats.close())

  for signal in (SIGINT, SIGTERM):
    loop.add_signal_handler(signal, handle_signal)

  for event in events:
    await nats.subscribe(event, cb=received_message)


def event_type(event):
  """Validate an event type argument."""
  # TODO: The believe is that there are actually more types that Polygon
  #       supports.
  if not event.startswith(("A.", "AM.", "Q.", "T.")):
    raise ArgumentTypeError(f"Unrecognized event type: {event}")

  return event


def main(args):
  """Subscribe to a set of streams, listen for events, and print them to stdout."""
  if POLYGON_API_KEY not in environ:
    print(f"{POLYGON_API_KEY} environment variable not set", file=stderr)
    return

  api_key = environ[POLYGON_API_KEY]

  parser = ArgumentParser(description=DESCRIPTION)
  parser.add_argument(
    "events", action="store", default=[], nargs="+", type=event_type,
    help="The events to subscribe to. Supported types are A.* & AM.*, T*., and "
         "Q.* for second aggregates, minute aggregates, trades, and quotes, "
         "respectively. E.g., T.MSFT " "will subscribe to trades of Microsoft "
         "stock.",
  )
  parser.add_argument(
    "-s", "--server", action="append", metavar="servers", nargs=1,
    dest="servers", required=True,
    help="A server to connect to and stream from. Can be supplied multiple "
         "times.",
  )
  ns = parser.parse_args(args)

  # The namespace's appended list arguments are stored as a list of list
  # of strings. Convert them to a list of strings.
  servers = [x for x, in ns.servers]

  loop = get_event_loop()
  loop.run_until_complete(run(servers, api_key, ns.events, loop))
  loop.run_forever()
  loop.close()


if __name__ == "__main__":
  main(argv[1:])
