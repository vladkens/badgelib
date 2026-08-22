use serde::{Deserialize, Deserializer, Serialize};

use crate::Error;

fn normalize_hex(value: &str) -> Option<String> {
  let value = value.trim().trim_start_matches('#').to_lowercase();
  if !matches!(value.len(), 3 | 6) || !value.chars().all(|character| character.is_ascii_hexdigit())
  {
    return None;
  }

  if value.len() == 3 {
    Some(value.chars().flat_map(|character| [character, character]).collect())
  } else {
    Some(value)
  }
}

/// Time period used by [`Badge::for_downloads`](crate::Badge::for_downloads).
pub enum Period {
  /// Downloads per week.
  Week,
  /// Downloads per month.
  Month,
  /// Downloads per year.
  Year,
  /// Total downloads without a period suffix.
  Total,
}

// MARK: Animation

/// An opinionated background animation applied to a badge.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Animation {
  /// Scrolls every configured label or value gradient.
  Flow,
  /// Sweeps a light streak across the full badge.
  Shine,
  /// Drifts softly blurred lights across the full badge.
  Aurora,
}

// MARK: Color

#[derive(Debug, Default, Clone, PartialEq)]
/// A badge background or logo color.
///
/// Named variants use the crate's built-in palette. Use [`Color::try_from`] to
/// validate a color name or a three- or six-digit hexadecimal value.
pub enum Color {
  /// Blue (`#3b82f6`), the default value color.
  #[default]
  Blue,
  /// Green (`#16a34a`).
  Green,
  /// Lime (`#65a30d`).
  Lime,
  /// Yellow (`#ca8a04`).
  Yellow,
  /// Orange (`#ea580c`).
  Orange,
  /// Red (`#ef4444`).
  Red,
  /// Gray (`#71717a`).
  Gray,
  /// Black (`#18181b`), the default label color.
  Black,
  /// White (`#f4f4f5`).
  White,
  /// Cyan (`#0891b2`).
  Cyan,
  /// A custom three- or six-digit hexadecimal color. Invalid values render as
  /// black; use [`Color::try_from`] to validate untrusted input.
  Hex(String),
}

impl Color {
  /// Returns the hexadecimal value without a leading `#`.
  pub fn to_hex(&self) -> String {
    // Exact Tailwind CSS v3 values; shades are selected individually.
    // https://tailwindcss.com/docs/colors
    match self {
      Color::Blue => "3b82f6".into(),   // blue-500
      Color::Green => "16a34a".into(),  // green-600
      Color::Lime => "65a30d".into(),   // lime-600
      Color::Yellow => "ca8a04".into(), // yellow-600
      Color::Orange => "ea580c".into(), // orange-600
      Color::Red => "ef4444".into(),    // red-500
      Color::Gray => "71717a".into(),   // zinc-500
      Color::Black => "18181b".into(),  // zinc-900
      Color::White => "f4f4f5".into(),  // zinc-100
      Color::Cyan => "0891b2".into(),   // cyan-600
      Color::Hex(hex) => normalize_hex(hex).unwrap_or_else(|| Color::Black.to_hex()),
    }
  }

  /// Returns the color as a CSS hexadecimal value with a leading `#`.
  pub fn to_css(&self) -> String {
    format!("#{}", self.to_hex())
  }
}

impl<'a> TryFrom<&'a str> for Color {
  type Error = Error;

  fn try_from(value: &'a str) -> Result<Self, Self::Error> {
    // https://github.com/badges/shields/blob/master/badge-maker/lib/color.js
    match value.to_lowercase().trim().replace("#", "").as_ref() {
      "blue" => Ok(Color::Blue),
      "green" | "brightgreen" | "bright-green" => Ok(Color::Green),
      "lime" | "yellowgreen" => Ok(Color::Lime),
      "yellow" => Ok(Color::Yellow),
      "orange" => Ok(Color::Orange),
      "red" => Ok(Color::Red),
      "gray" | "grey" => Ok(Color::Gray),
      "black" => Ok(Color::Black),
      "white" => Ok(Color::White),
      "cyan" => Ok(Color::Cyan),
      x => {
        if (x.len() == 3 || x.len() == 6) && x.chars().all(|c| c.is_ascii_hexdigit()) {
          Ok(Color::Hex(x.to_string()))
        } else {
          Err(Error::InvalidColor(x.to_string()))
        }
      }
    }
  }
}

impl<'de> Deserialize<'de> for Color {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Self::try_from(s.as_ref()).map_err(serde::de::Error::custom)
  }
}

impl Serialize for Color {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&self.to_hex())
  }
}

// MARK: Style

/// Visual treatment applied to the badge background and corners.
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
  /// Rounded corners with a subtle vertical highlight.
  #[default]
  Flat,
  /// Square corners with solid backgrounds and no highlight.
  FlatSquare,
  /// Large square badge with uppercase text and a bold value.
  ForTheBadge,
}

impl<'a> TryFrom<&'a str> for Style {
  type Error = String;

  fn try_from(value: &'a str) -> Result<Self, Self::Error> {
    match value.to_lowercase().replace("-", "").trim() {
      "flat" => Ok(Style::Flat),
      "flatsquare" => Ok(Style::FlatSquare),
      "forthebadge" => Ok(Style::ForTheBadge),
      x => Err(format!("unknown badge style '{}'", x)),
    }
  }
}

impl<'de> Deserialize<'de> for Style {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Self::try_from(s.as_ref()).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_invalid_color_uses_library_error() {
    let error = Color::try_from("not-a-color").unwrap_err();
    assert!(matches!(&error, Error::InvalidColor(color) if color == "not-a-color"));
    assert_eq!(error.to_string(), "invalid color 'not-a-color'");
  }

  #[test]
  fn test_invalid_direct_hex_falls_back_to_black() {
    assert_eq!(Color::Hex("not-a-color".into()).to_hex(), Color::Black.to_hex());
  }

  #[test]
  fn test_builtin_palette() {
    for (color, expected) in [
      (Color::Blue, "3b82f6"),
      (Color::Green, "16a34a"),
      (Color::Lime, "65a30d"),
      (Color::Yellow, "ca8a04"),
      (Color::Orange, "ea580c"),
      (Color::Red, "ef4444"),
      (Color::Gray, "71717a"),
      (Color::Black, "18181b"),
      (Color::White, "f4f4f5"),
      (Color::Cyan, "0891b2"),
    ] {
      assert_eq!(color.to_hex(), expected);
    }
  }

  #[test]
  fn test_bright_green_aliases() {
    assert_eq!(Color::try_from("brightgreen").unwrap(), Color::Green);
    assert_eq!(Color::try_from("bright-green").unwrap(), Color::Green);
  }
}

// MARK: Format

#[cfg(feature = "axum")]
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Format {
  #[default]
  Svg,
  Json,
}

#[cfg(feature = "axum")]
impl<'a> TryFrom<&'a str> for Format {
  type Error = String;

  fn try_from(value: &'a str) -> Result<Self, Self::Error> {
    match value.to_lowercase().trim() {
      "svg" => Ok(Format::Svg),
      "json" => Ok(Format::Json),
      x => Err(format!("unknown badge format '{}'", x)),
    }
  }
}

#[cfg(feature = "axum")]
impl<'de> Deserialize<'de> for Format {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Self::try_from(s.as_ref()).map_err(serde::de::Error::custom)
  }
}
