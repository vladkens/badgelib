## v0.5.2 – 2026-08-28

### Improvements

- Reduced the downstream dependency tree by removing Maud from SVG rendering

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.5.1...v0.5.2

---

## v0.5.1 – 2026-08-26

### Features

- Added support for building `badgelib` for `wasm32-unknown-unknown`, including with the optional `simple-icons` and `axum` features

### Improvements

- Reduced the dependency graph and compile overhead for client applications, especially when using the optional Axum integration

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.5.0...v0.5.1

---

## v0.5.0 – 2026-08-22

### Breaking Changes

- Made named Simple Icons opt-in through the `simple-icons` feature; applications using named logos must now enable it
- Changed `Color::try_from` to return the public `Error` type instead of a string

### Features

- Added the `ForTheBadge` style with larger uppercase text and bold values
- Added `Badge::try_logo` for rejecting unknown Simple Icons slugs

### Fixes

- Exposed `Style` publicly and fixed `FlatSquare` badges to use square corners without a background highlight
- Fixed `brightgreen` and `bright-green` color aliases
- Fixed size badges for PiB, EiB, and the full `u64` range
- Fixed rating and star badges for invalid or out-of-range scores
- Fixed default Simple Icons colors to follow the badge text color

### Improvements

- Improved contrast for the built-in color palette
- Improved custom hexadecimal color handling with normalization and a safe fallback for invalid direct values

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.4.0...v0.5.0

---

## v0.4.0 – 2026-08-11

### Features

- Added adaptive precision for compact counts across `k`, `M`, `B`, and `T` scales
- Added license-aware badge colors for permissive, copyleft, public-domain, and unknown licenses
- Added support for Simple Icons slugs removed or renamed in historical releases

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.3.0...v0.4.0

---

## v0.3.0 – 2026-08-07

### Features

- Added custom SVG icons with support for multicolor artwork
- Added Flow, Shine, and Aurora animation presets through a unified animation API with reduced-motion support

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.2.0...v0.3.0

---

## v0.2.0 – 2026-07-22

### Features

- Added configurable left-to-right gradient backgrounds for label and value sections

### Docs

- Added public API documentation on docs.rs

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.1.1...v0.2.0

---

## v0.1.1 – 2025-08-31

### Maintenance

- Updated Rust dependencies

**Full Changelog**: https://github.com/vladkens/badgelib/compare/v0.1.0...v0.1.1

---

## v0.1.0 – 2025-04-10

### Features

- Released a Rust API for generating customizable SVG badges with labels, values, colors, and icons
- Added optional Axum response integration

**Full Changelog**: https://github.com/vladkens/badgelib/commits/v0.1.0
