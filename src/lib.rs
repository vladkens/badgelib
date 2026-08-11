//! Generate customizable SVG badges.
//!
//! Build a [`Badge`] by setting its label, value, solid or gradient backgrounds,
//! and optional custom icon, then render it with [`Badge::to_svg`]. Helpers
//! such as [`Badge::for_version`] and [`Badge::for_downloads`] provide common
//! badge formats and colors. Enable the `simple-icons` feature to select logos
//! by their [Simple Icons](https://simpleicons.org) slugs.
//!
//! # Example
//!
//! ```
//! use badgelib::{Badge, Color};
//!
//! let svg = Badge::new()
//!   .label("build")
//!   .value("passing")
//!   .value_color(Color::Green)
//!   .to_svg();
//!
//! assert!(svg.starts_with("<svg"));
//! ```

#[cfg(feature = "simple-icons")]
pub(crate) mod _icons;
pub(crate) mod _width;
pub(crate) mod badge;
pub(crate) mod error;
pub(crate) mod param;
pub(crate) mod utils;

#[doc(inline)]
pub use badge::Badge;
pub use error::{Error, Result};
#[doc(inline)]
pub use param::{Animation, Color, Period};
