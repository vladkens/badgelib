//! Generate customizable SVG badges.
//!
//! Build a [`Badge`] by setting its label, value, solid or gradient backgrounds,
//! and optional logo, then render it with [`Badge::to_svg`]. Helpers such as
//! [`Badge::for_version`] and [`Badge::for_downloads`] provide common badge
//! formats and colors.
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
//!   .logo("rust")
//!   .to_svg();
//!
//! assert!(svg.starts_with("<svg"));
//! ```

pub(crate) mod _icons;
pub(crate) mod _width;
pub(crate) mod badge;
pub(crate) mod param;
pub(crate) mod utils;

#[doc(inline)]
pub use badge::Badge;
#[doc(inline)]
pub use param::{Color, Period};
