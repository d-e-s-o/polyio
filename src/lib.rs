// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

mod events;
mod stock;
mod stream;

use std::borrow::Cow;

type Str = Cow<'static, str>;
