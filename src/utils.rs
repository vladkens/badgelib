use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::Deserialize;
use serde::de::IntoDeserializer;

use super::_width::WIDTHS;
use super::Color;
#[cfg(feature = "simple-icons")]
use crate::_icons::ICONS;

// https://github.com/serde-rs/serde/issues/1425#issuecomment-462282398
// note: "default" should be used with Option<T> to work, example:
// #[serde(deserialize_with = "empty_string_as_none", default)]
pub(crate) fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
  D: serde::Deserializer<'de>,
  T: serde::Deserialize<'de>,
{
  let opt: Option<String> = Option::deserialize(de)?;
  match opt.as_deref() {
    None | Some("") => Ok(None),
    Some(s) => T::deserialize(s.into_deserializer()).map(Some),
  }
}

pub fn cacl_width(text: &str) -> f32 {
  let fallback_width = WIDTHS[64]; // Width as "@" for overflows
  let mut total_width = 0.0;
  for ch in text.chars() {
    let index = ch as usize;
    let width = WIDTHS.get(index).copied().unwrap_or(fallback_width);
    total_width += width;
  }

  total_width
}

// pub fn to_min_ver(version: &str) -> String {
//   version.replace(">=", "≥").replace("<=", "≤")
// }

pub fn millify(n: u64) -> String {
  let mut n = n as f64;
  let mut i = 0;
  let units = ["", "k", "M", "B", "T"];
  while n >= 1_000.0 && i < units.len() - 1 {
    n /= 1_000.0;
    i += 1;
  }

  let label = if n >= 100.0 { format!("{n:.0}") } else { format!("{n:.1}") };
  let label = label.strip_suffix(".0").unwrap_or(&label);
  let label = format!("{label}{}", units[i]);
  label
}

// https://www.npmjs.com/package/byte-size
pub fn millify_iec(n: u64) -> String {
  let mut n = n as f64;
  let mut i = 0;
  let units = ["", "KiB", "MiB", "GiB", "TiB"];
  while n >= 1_024.0 {
    n /= 1_024.0;
    i += 1;
  }

  let label = format!("{n:.1}");
  let label = label.strip_suffix(".0").unwrap_or(&label);
  let label = format!("{label} {}", units[i]);
  label
}

pub(crate) fn to_icon_uri(svg: &str) -> String {
  format!("data:image/svg+xml;base64,{}", BASE64_STANDARD.encode(svg))
}

#[cfg(feature = "simple-icons")]
pub(crate) fn get_icon(name: &str, color: &Option<Color>) -> Option<String> {
  let pretenders = [
    name.to_lowercase(),
    name.to_lowercase().replace('-', "").replace("!", "").replace("_", "").replace(" ", ""),
    name.to_lowercase().replace('.', "dot").replace("+", "plus"),
  ];

  let icon = format!(
    r#"<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="0 0 24 24" fill="{}"><path d="{}" /></svg>"#,
    color.clone().map(|x| x.to_css()).unwrap_or("#fff".into()),
    pretenders.iter().find_map(|n| ICONS.get(n))?
  );

  Some(to_icon_uri(&icon))
}

pub(crate) fn text_color(bg_color: &Color) -> Color {
  let hex = bg_color.to_hex();
  let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
  let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
  let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;

  // Using relative luminance formula
  let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  // println!("Luminance: {}", luminance);
  if luminance > 0.85 { Color::Black } else { Color::White }
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
enum LicenseKind {
  Unknown,
  Copyleft,
  Permissive,
  PublicDomain,
}

fn license_kind(value: &str) -> LicenseKind {
  match value.trim() {
    "AFL-3.0" | "APACHE-2.0" | "APACHE 2.0" | "APACHE LICENSE 2.0" | "ARTISTIC-2.0" | "BSD"
    | "BSD-2-CLAUSE" | "BSD-3-CLAUSE" | "BSD-3-CLAUSE-CLEAR" | "BSL-1.0" | "CC-BY-4.0"
    | "ECL-2.0" | "ISC" | "MIT" | "MIT LICENSE" | "MS-PL" | "NCSA" | "POSTGRESQL" | "ZLIB" => {
      LicenseKind::Permissive
    }
    "AGPL-1.0-ONLY" | "AGPL-1.0-OR-LATER" | "AGPL-3.0" | "AGPL-3.0-ONLY" | "AGPL-3.0-OR-LATER"
    | "AGPLV3+" | "CC-BY-SA-4.0" | "EPL" | "EPL-1.0" | "EPL-2.0" | "EUPL-1.1" | "GPL"
    | "GPL-1.0-ONLY" | "GPL-1.0-OR-LATER" | "GPL-2.0" | "GPL-2.0-ONLY" | "GPL-2.0-OR-LATER"
    | "GPL-3.0" | "GPL-3.0-ONLY" | "GPL-3.0-OR-LATER" | "GPLV2" | "GPLV2+" | "GPLV3" | "GPLV3+"
    | "LGPL" | "LGPL-2.0-ONLY" | "LGPL-2.0-OR-LATER" | "LGPL-2.1" | "LGPL-2.1-ONLY"
    | "LGPL-2.1-OR-LATER" | "LGPL-3.0" | "LGPL-3.0-ONLY" | "LGPL-3.0-OR-LATER" | "LGPLV2"
    | "LGPLV2+" | "LGPLV3" | "LGPLV3+" | "LPPL-1.3C" | "MPL" | "MPL 1.1" | "MPL 2.0"
    | "MPL-2.0" | "MS-RL" | "OFL-1.1" | "OSL-3.0" => LicenseKind::Copyleft,
    "0BSD" | "CC0" | "CC0-1.0" | "UNLICENSE" | "WTFPL" => LicenseKind::PublicDomain,
    _ => LicenseKind::Unknown,
  }
}

pub(crate) fn license_color(license: &str) -> Color {
  let normalized = license.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase();
  let normalized = normalized.replace(" OR ", "|").replace(" AND ", "|").replace(" WITH ", "|");
  let kind = normalized
    .split(['|', ',', '/', ';', '&', '(', ')'])
    .flat_map(|part| std::iter::once(part).chain(part.split_ascii_whitespace()))
    .map(license_kind)
    .max()
    .unwrap_or(LicenseKind::Unknown);

  match kind {
    LicenseKind::Permissive => Color::Blue,
    LicenseKind::Copyleft => Color::Orange,
    LicenseKind::PublicDomain => Color::Lime,
    LicenseKind::Unknown => Color::Gray,
  }
}

pub(crate) fn rating_color<T: Into<f64>>(value: T, max_value: T) -> Color {
  let (value, max_value) = (value.into(), max_value.into());

  match value {
    x if x >= max_value * 0.80 => Color::Green,
    x if x >= max_value * 0.60 => Color::Lime,
    x if x >= max_value * 0.40 => Color::Yellow,
    x if x >= max_value * 0.20 => Color::Orange,
    _ => Color::Red,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_millify_uses_adaptive_precision_for_every_scale() {
    for (value, expected) in [
      (1_200, "1.2k"),
      (12_300, "12.3k"),
      (110_400, "110k"),
      (1_200_000, "1.2M"),
      (12_300_000, "12.3M"),
      (110_400_000, "110M"),
      (1_200_000_000, "1.2B"),
      (12_300_000_000, "12.3B"),
      (110_400_000_000, "110B"),
      (1_200_000_000_000, "1.2T"),
      (12_300_000_000_000, "12.3T"),
      (110_400_000_000_000, "110T"),
    ] {
      assert_eq!(millify(value), expected);
    }
  }

  #[test]
  fn test_millify_scale_boundaries() {
    for (value, expected) in [
      (0, "0"),
      (999, "999"),
      (1_000, "1k"),
      (999_000, "999k"),
      (1_000_000, "1M"),
      (999_000_000, "999M"),
      (1_000_000_000, "1B"),
      (999_000_000_000, "999B"),
      (1_000_000_000_000, "1T"),
      (999_000_000_000_000, "999T"),
    ] {
      assert_eq!(millify(value), expected);
    }
  }
}
