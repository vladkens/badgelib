# badgelib

<div align="center">

[<img src="https://badges.ws/crates/v/badgelib?color=f74d02&logo=rust" alt="version" />](https://crates.io/crates/badgelib)
[<img src="https://badges.ws/crates/docs/badgelib?logo=rust" alt="docs" />](https://docs.rs/badgelib)
[<img src="https://badges.ws/crates/dt/badgelib?logo=rust" alt="downloads" />](https://crates.io/crates/badgelib)
[<img src="https://badges.ws/github/license/vladkens/badgelib?logo=opensourceinitiative" alt="license" />](https://github.com/vladkens/badgelib/blob/main/LICENSE)
[<img src="https://badges.ws/badge/-/buy%20me%20a%20coffee/ff813f?icon=buymeacoffee&label" alt="donate" />](https://buymeacoffee.com/vladkens)

</div>

`badgelib` is a Rust library for rendering customizable SVG badges directly in your application. Use it when you want to generate badges without running a separate badge service. It powers [badges.ws](https://badges.ws); the web service and its data integrations live in the main [badges repository](https://github.com/vladkens/badges).

## Install

```sh
cargo add badgelib
```

## Quick start

```rust
use badgelib::{Badge, Color};

fn main() -> std::io::Result<()> {
  let svg = Badge::new()
    .label("build")
    .value("passing")
    .value_color(Color::Green)
    .logo("rust")
    .radius(4)
    .to_svg();

  std::fs::write("badge.svg", svg)
}
```

`Badge::to_svg()` returns the complete SVG document as a `String`. Badges can include separate label and value colors, an icon from [Simple Icons](https://simpleicons.org), a custom icon color, and a border radius.

## Built-in badge types

The library includes helpers for common badge values and their default formatting:

```rust
use badgelib::{Badge, Period};

let version = Badge::new().for_version("version", "1.2.0");
let license = Badge::new().for_license("MIT");
let downloads = Badge::new().for_downloads(Period::Month, 1_234_567);
let build = Badge::new().for_ci_status("build", true);
let size = Badge::new().for_size("size", 1_234_567);
let rating = Badge::new().for_rating("rating", 4.5, 5.0);
```

Available helpers cover versions, licenses, downloads, CI status, counts, sizes, ratings, star ratings, and relative durations. You can override their labels and colors with the regular builder methods.

## JSON output

Use `Badge::to_json()` when you need the badge parameters as JSON instead of rendered SVG:

```rust
let json = badgelib::Badge::new()
  .label("version")
  .value("v1.2.0")
  .to_json();
```

## Axum integration

Enable the optional `axum` feature to return a `Badge` directly from a handler:

```sh
cargo add badgelib --features axum
```

```rust
use badgelib::Badge;

async fn version_badge() -> Badge {
  Badge::new().for_version("version", env!("CARGO_PKG_VERSION"))
}
```

With this feature enabled, `Badge` implements Axum's `IntoResponse` and returns SVG by default with the appropriate content type and cache headers.

## Development

Clone the repository with its Simple Icons submodule, then run the existing checks:

```sh
git clone --recurse-submodules https://github.com/vladkens/badgelib.git
cd badgelib
make check
make test
```

## Credits

- Inspired by [Shields.io](https://shields.io) and [Badgen](https://badgen.net).
- Icons are provided by [Simple Icons](https://simpleicons.org).
- Badge text is rendered with [DejaVu Sans](https://dejavu-fonts.github.io).

## License

Distributed under the [MIT License](LICENSE).
