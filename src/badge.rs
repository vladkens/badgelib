use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Color;
#[cfg(feature = "simple-icons")]
use crate::Error;
#[cfg(feature = "axum")]
use crate::param::Format;
use crate::param::{Animation, Period, Style};
#[cfg(all(test, feature = "simple-icons"))]
use crate::utils::get_icon;
#[cfg(feature = "simple-icons")]
use crate::utils::has_icon;
#[cfg(test)]
use crate::utils::to_icon_uri;
use crate::utils::{empty_string_as_none, license_color, millify, millify_iec, rating_color};

#[path = "render.rs"]
mod render;

#[cfg(feature = "axum")]
fn default_cache() -> u32 {
  86400 // 24 hours
}

fn deserialize_gradient<'de, D>(deserializer: D) -> Result<Option<Vec<Color>>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let colors = Option::<Vec<Color>>::deserialize(deserializer)?;
  if colors.as_ref().is_some_and(|colors| colors.len() < 2) {
    return Err(serde::de::Error::custom("a gradient requires at least two colors"));
  }
  Ok(colors)
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
/// A configurable badge that can be rendered as SVG or JSON.
///
/// Builder methods consume and return the badge, so they can be chained.
/// Without customization, labels use black and values use blue.
pub struct Badge {
  #[serde(rename = "label")]
  label: Option<String>,

  #[serde(rename = "labelColor", deserialize_with = "empty_string_as_none", default)]
  label_color: Option<Color>,

  #[serde(
    rename = "labelGradient",
    deserialize_with = "deserialize_gradient",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  label_gradient: Option<Vec<Color>>,

  #[serde(rename = "value")]
  value: Option<String>,

  #[serde(rename = "color", deserialize_with = "empty_string_as_none", default)]
  value_color: Option<Color>,

  #[serde(
    rename = "gradient",
    deserialize_with = "deserialize_gradient",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  value_gradient: Option<Vec<Color>>,

  #[cfg(feature = "simple-icons")]
  #[serde(rename = "logo", alias = "icon")]
  logo: Option<String>,

  #[cfg(feature = "simple-icons")]
  #[serde(rename = "logoColor", alias = "iconColor")]
  #[serde(deserialize_with = "empty_string_as_none", default)]
  logo_color: Option<Color>,

  /// Custom icon markup set via [`Badge::icon_svg`].
  #[serde(skip)]
  icon_svg: Option<String>,

  /// The background animation selected via [`Badge::animation`].
  #[serde(default, skip_serializing_if = "Option::is_none")]
  animation: Option<Animation>,

  #[serde(rename = "radius")]
  radius: Option<u8>,

  #[serde(rename = "style", default = "Default::default")]
  style: Style,

  #[cfg(feature = "axum")]
  #[serde(rename = "format", default = "Default::default")]
  format: Format,

  #[cfg(feature = "axum")]
  #[serde(rename = "cache", default = "default_cache")]
  cache: u32,
}

impl Badge {
  /// Creates a badge with default colors and no text or logo.
  pub fn new() -> Self {
    Self::default()
  }

  // MARK: Setters

  /// Sets the text shown on the left side of the badge.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(label.into());
    self
  }

  /// Sets the background color of the label.
  pub fn label_color(mut self, color: Color) -> Self {
    self.label_color = Some(color);
    self.label_gradient = None;
    self
  }

  /// Sets a left-to-right gradient background for the label.
  ///
  /// Colors are distributed evenly and at least two are required.
  ///
  /// # Panics
  ///
  /// Panics when fewer than two colors are provided.
  pub fn label_gradient(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
    let colors = colors.into_iter().collect::<Vec<_>>();
    assert!(colors.len() >= 2, "a gradient requires at least two colors");
    self.label_color = None;
    self.label_gradient = Some(colors);
    self
  }

  /// Sets the text shown on the right side of the badge.
  pub fn value(mut self, value: &str) -> Self {
    self.value = Some(value.into());
    self
  }

  /// Sets the background color of the value.
  pub fn value_color(mut self, color: Color) -> Self {
    self.value_color = Some(color);
    self.value_gradient = None;
    self
  }

  /// Sets a left-to-right gradient background for the value.
  ///
  /// Colors are distributed evenly and at least two are required.
  ///
  /// # Panics
  ///
  /// Panics when fewer than two colors are provided.
  pub fn value_gradient(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
    let colors = colors.into_iter().collect::<Vec<_>>();
    assert!(colors.len() >= 2, "a gradient requires at least two colors");
    self.value_color = None;
    self.value_gradient = Some(colors);
    self
  }

  /// Selects one opinionated background animation. Calling this method again
  /// replaces the previously selected animation.
  pub fn animation(mut self, animation: Animation) -> Self {
    self.animation = Some(animation);
    self
  }

  /// Adds a logo by its [Simple Icons](https://simpleicons.org) slug.
  /// Available with the `simple-icons` feature.
  #[cfg(feature = "simple-icons")]
  pub fn logo(mut self, logo: &str) -> Self {
    self.logo = Some(logo.into());
    self.icon_svg = None;
    self
  }

  /// Adds a logo only if its Simple Icons slug exists.
  /// Available with the `simple-icons` feature.
  ///
  /// # Errors
  ///
  /// Returns [`Error::UnknownLogo`] when the slug does not exist.
  #[cfg(feature = "simple-icons")]
  pub fn try_logo(self, logo: &str) -> crate::Result<Self> {
    if !has_icon(logo) {
      return Err(Error::UnknownLogo(logo.into()));
    }
    Ok(self.logo(logo))
  }

  /// Sets the logo color. By default, the logo matches the text in its segment.
  /// Available with the `simple-icons` feature.
  #[cfg(feature = "simple-icons")]
  pub fn logo_color(mut self, color: Color) -> Self {
    self.logo_color = Some(color);
    self
  }

  /// Sets a custom icon from raw, already-colored SVG markup. Use this for
  /// icons that need multiple colors or shapes. The caller is responsible for
  /// the markup; it is emitted as-is, so don't pass it untrusted input.
  pub fn icon_svg(mut self, svg: impl Into<String>) -> Self {
    self.icon_svg = Some(svg.into());
    #[cfg(feature = "simple-icons")]
    {
      self.logo = None;
    }
    self
  }

  /// Sets the corner radius, clamped to `12` during rendering.
  pub fn radius(mut self, radius: u8) -> Self {
    self.radius = Some(radius);
    self
  }

  /// Selects the badge's background and corner style.
  ///
  /// [`Style::Flat`](crate::Style::Flat) is used by default. An explicit
  /// [`Badge::radius`] overrides the style's default corner radius without
  /// changing its background treatment. Use
  /// [`Style::ForTheBadge`](crate::Style::ForTheBadge) for a larger badge with
  /// uppercase text and a bold value.
  ///
  /// # Examples
  ///
  /// ```
  /// use badgelib::{Badge, Style};
  ///
  /// let svg = Badge::new()
  ///   .label("build")
  ///   .value("passing")
  ///   .style(Style::ForTheBadge)
  ///   .to_svg();
  /// ```
  pub fn style(mut self, style: Style) -> Self {
    self.style = style;
    self
  }

  // MARK: Predefined

  /// Configures a version badge and chooses a color based on the version.
  ///
  /// A missing version becomes `unknown`, and versions without a leading `v`
  /// receive one. Pre-release versions are cyan and `v0` versions are orange.
  pub fn for_version(mut self, label: &str, value: &str) -> Self {
    let value = match value.to_lowercase().trim() {
      "" | "unknown" | "none" => "unknown".into(),
      x if x.starts_with('v') => x.into(),
      x => format!("v{x}"),
    };

    let color = match &value {
      x if x.contains("alpha")
        || x.contains("beta")
        || x.contains("canary")
        || x.contains("rc")
        || x.contains("dev") =>
      {
        Color::Cyan
      }
      x if x.starts_with("v0.") => Color::Orange,
      _ => Color::Blue,
    };

    self.label = self.label.or(Some(label.into()));
    self.value = Some(value);
    if self.value_color.is_none() && self.value_gradient.is_none() {
      self.value_color = Some(color);
    }
    self
  }

  /// Configures a license badge with a color based on the license family.
  pub fn for_license(mut self, license: &str) -> Self {
    self.label = self.label.or(Some("license".into()));
    self.value = Some(license.into());
    if self.value_color.is_none() && self.value_gradient.is_none() {
      self.value_color = Some(license_color(license));
    }
    self
  }

  /// Configures a green downloads badge with a compact count.
  pub fn for_downloads(mut self, period: Period, value: u64) -> Self {
    let value = match period {
      Period::Week => format!("{}/week", millify(value)),
      Period::Month => format!("{}/month", millify(value)),
      Period::Year => format!("{}/year", millify(value)),
      Period::Total => millify(value),
    };

    self.label = self.label.or(Some("downloads".into()));
    self.value = Some(value);
    if self.value_color.is_none() && self.value_gradient.is_none() {
      self.value_color = Some(Color::Green);
    }
    self
  }

  /// Configures a CI badge as green `passing` or red `failing`.
  pub fn for_ci_status(mut self, label: &str, status: bool) -> Self {
    let value = if status { "passing" } else { "failing" };
    let color = if status { Color::Green } else { Color::Red };

    self.label = self.label.or(Some(label.into()));
    self.value = Some(value.into());
    self.value_color = Some(color);
    self.value_gradient = None;
    self
  }

  /// Configures a blue badge with a compact decimal count such as `1.2k`.
  pub fn for_count(mut self, label: &str, value: u64) -> Self {
    self.label = self.label.or(Some(label.into()));
    self.value = Some(millify(value));
    self.value_color = Some(Color::Blue);
    self.value_gradient = None;
    self
  }

  /// Configures a blue badge with an IEC byte size such as `1.2 MiB`.
  pub fn for_size(mut self, label: &str, value: u64) -> Self {
    self.label = self.label.or(Some(label.into()));
    self.value = Some(millify_iec(value));
    self.value_color = Some(Color::Blue);
    self.value_gradient = None;
    self
  }

  /// Configures a numeric rating badge with a color based on the score.
  pub fn for_rating(mut self, label: &str, value: f64, max_value: f64) -> Self {
    self.label = self.label.or(Some(label.into()));
    self.value = Some(format!("{:.1}/{}", value, max_value));
    self.value_color = Some(rating_color(value, max_value));
    self.value_gradient = None;
    self
  }

  /// Configures a five-star rating badge with a color based on the score.
  pub fn for_stars(mut self, label: &str, value: f64, max_value: f64) -> Self {
    let stars = {
      let score = if value.is_finite() && max_value.is_finite() && max_value > 0.0 {
        (value / max_value * 5.0).clamp(0.0, 5.0)
      } else {
        0.0
      };

      // unfortunately not supported yet https://symbl.cc/en/2BE8/
      let full_part = "★".repeat(score as usize);
      let half_part = if score.fract() >= 0.5 { "½" } else { "" };
      let mut line = format!("{}{}", full_part, half_part);

      let size = line.chars().count();
      if size < 5 {
        line.push_str(&"☆".repeat(5 - size));
      }

      line
    };

    self.label = self.label.or(Some(label.into()));
    self.value = Some(stars);
    self.value_color = Some(rating_color(value, max_value));
    self.value_gradient = None;
    self
  }

  /// Configures a relative-time badge for a UTC timestamp.
  pub fn for_duration(mut self, label: &str, value: DateTime<Utc>) -> Self {
    let days = Utc::now().signed_duration_since(value).num_days();
    let (value, color) = match days {
      0 => ("today".into(), Color::Green),
      1 => ("yesterday".into(), Color::Green),
      2..=7 => (format!("{} days ago", days), Color::Green),
      8..=30 => (format!("{} days ago", days), Color::Lime),
      31..=180 => (format!("{} months ago", days / 30), Color::Yellow),
      181..=365 => (format!("{} months ago", days / 30), Color::Orange),
      _ => (format!("{} years ago", days / 365), Color::Red),
    };

    self.label = self.label.or(Some(label.into()));
    self.value = Some(value);
    self.value_color = Some(color); // Changed from Color::Blue to use the calculated color
    self.value_gradient = None;
    self
  }

  // MARK: Render

  /// Serializes the badge configuration as JSON.
  pub fn to_json(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  /// Renders the badge as a complete SVG document.
  pub fn to_svg(&self) -> String {
    render::svg(self)
  }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for Badge {
  fn into_response(self) -> axum::response::Response {
    let cc = format!("public,max-age={0},s-maxage=300,stale-while-revalidate={0}", self.cache);
    let (ct, content) = match self.format {
      Format::Svg => ("image/svg+xml", self.to_svg()),
      Format::Json => ("application/json", self.to_json()),
    };

    let rep = (
      axum::http::StatusCode::OK,
      [(axum::http::header::CACHE_CONTROL, cc), (axum::http::header::CONTENT_TYPE, ct.into())],
      content,
    );

    rep.into_response()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_style_controls_corners_and_highlight() {
    let flat = Badge::new().value("passing").to_svg();
    assert!(flat.contains(r#"id="s""#));
    assert!(flat.contains(r#"fill="url(#s)""#));

    let square = Badge::new().value("passing").style(Style::FlatSquare).to_svg();
    assert!(!square.contains(r#"id="s""#));
    assert!(!square.contains(r#"fill="url(#s)""#));
    assert!(square.contains(r#"rx="0""#));
  }

  #[test]
  fn test_value_gradient() {
    let badge = Badge::new().label("build").value("passing").value_gradient([
      Color::Red,
      Color::Orange,
      Color::Cyan,
    ]);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="vg""#));
    assert!(svg.contains(r##"offset="0.000%" stop-color="#ef4444""##));
    assert!(svg.contains(r##"offset="50.000%" stop-color="#ea580c""##));
    assert!(svg.contains(r##"offset="100.000%" stop-color="#0891b2""##));
    assert!(svg.contains(r#"fill="url(#vg)""#));
    assert_eq!(badge.value_color, None);
    assert!(badge.to_json().contains(r#""gradient":["ef4444","ea580c","0891b2"]"#));
  }

  #[test]
  fn test_color_and_gradient_replace_each_other() {
    let solid = Badge::new().value_gradient([Color::Red, Color::Blue]).value_color(Color::Green);
    assert_eq!(solid.value_color, Some(Color::Green));
    assert_eq!(solid.value_gradient, None);

    let gradient = Badge::new().value_color(Color::Green).value_gradient([Color::Red, Color::Blue]);
    assert_eq!(gradient.value_color, None);
    assert_eq!(gradient.value_gradient, Some(vec![Color::Red, Color::Blue]));
  }

  #[test]
  #[should_panic(expected = "a gradient requires at least two colors")]
  fn test_gradient_requires_two_colors() {
    Badge::new().value_gradient([Color::Red]);
  }

  #[test]
  fn test_gradient_deserialization_requires_two_colors() {
    let result = serde_json::from_str::<Badge>(r#"{"gradient":["red"]}"#);
    assert!(result.unwrap_err().to_string().starts_with("a gradient requires at least two colors"));
  }

  #[test]
  fn test_flow_animates_value_gradient() {
    let badge = Badge::new()
      .label("build")
      .value("flowing")
      .value_gradient([Color::Red, Color::Blue])
      .animation(Animation::Flow);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="vg""#));
    assert!(svg.contains("<animateTransform"));
    assert!(svg.contains(r#"type="translate""#));
    assert!(svg.contains(r##"offset="0.000%" stop-color="#ef4444""##));
    assert!(svg.contains(r##"offset="25.000%" stop-color="#3b82f6""##));
    assert!(svg.contains(r##"offset="50.000%" stop-color="#ef4444""##));
    assert!(svg.contains(r##"offset="75.000%" stop-color="#3b82f6""##));
    assert!(svg.contains(r##"offset="100.000%" stop-color="#ef4444""##));
    assert!(svg.contains(r#"id="vgs""#));
    assert!(svg.contains("prefers-reduced-motion: reduce"));
  }

  #[test]
  fn test_gradient_not_animated_by_default() {
    let badge = Badge::new().value("build").value_gradient([Color::Red, Color::Blue]);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="vg""#));
    assert!(!svg.contains("animateTransform"));
  }

  #[test]
  fn test_flow_without_gradient_has_no_effect() {
    let badge = Badge::new().value("build").value_color(Color::Green).animation(Animation::Flow);
    let svg = badge.to_svg();

    assert!(!svg.contains(r#"id="vg""#));
    assert!(!svg.contains("animateTransform"));
  }

  #[test]
  fn test_flow_animates_label_gradient() {
    let badge = Badge::new()
      .label("flowing")
      .value("build")
      .label_gradient([Color::Red, Color::Blue])
      .animation(Animation::Flow);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="lg""#));
    assert!(svg.contains("<animateTransform"));
  }

  #[test]
  fn test_flow_animates_both_gradients_at_once() {
    let badge = Badge::new()
      .label("a")
      .label_gradient([Color::Red, Color::Blue])
      .value("b")
      .value_gradient([Color::Cyan, Color::Orange])
      .animation(Animation::Flow);
    let svg = badge.to_svg();

    assert_eq!(svg.matches("<animateTransform").count(), 2);
  }

  #[test]
  fn test_shine() {
    let badge = Badge::new().value("passing").animation(Animation::Shine);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="sh""#));
    assert!(svg.contains(r#"fill="url(#sh)""#));
    assert!(svg.contains(r#"attributeName="x""#));
    assert!(svg.contains(r#"keyTimes="0;0.25;0.70;1""#));
    assert!(svg.contains(r#"dur="4s""#));
    assert!(svg.contains(r#"class="shine""#));
    assert!(svg.contains(".shine{display:none}"));
  }

  #[test]
  fn test_shine_off_by_default() {
    let badge = Badge::new().value("passing");
    let svg = badge.to_svg();

    assert!(!svg.contains(r#"id="sh""#));
  }

  #[test]
  fn test_aurora_derives_a_palette_and_respects_reduced_motion() {
    let svg =
      Badge::new().value("aurora").value_color(Color::Blue).animation(Animation::Aurora).to_svg();

    assert!(svg.contains(r#"id="aurora-blur""#));
    assert!(svg.contains(r#"class="aurora-motion""#));
    assert!(svg.contains(r#"class="aurora-static""#));
    assert!(svg.contains("feGaussianBlur"));
    assert!(svg.contains(".aurora-motion{display:none}"));
  }

  #[test]
  fn test_aurora_is_off_by_default() {
    assert!(!Badge::new().value("plain").to_svg().contains("aurora-blur"));
  }

  #[test]
  fn test_animation_is_replaced_and_round_trips_through_json() {
    let badge = Badge::new()
      .value("passing")
      .value_gradient([Color::Red, Color::Blue])
      .animation(Animation::Flow)
      .animation(Animation::Shine)
      .animation(Animation::Aurora);
    let json = badge.to_json();
    let restored: Badge = serde_json::from_str(&json).unwrap();

    assert!(json.contains(r#""animation":"aurora""#));
    assert_eq!(restored.animation, Some(Animation::Aurora));
  }

  #[test]
  fn test_icon_svg_is_rendered() {
    let raw = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M1 2" fill="#facc15" /></svg>"##;
    let badge = Badge::new().value("custom").icon_svg(raw);
    let svg = badge.to_svg();

    assert!(svg.contains(&to_icon_uri(raw)));
  }

  #[cfg(feature = "simple-icons")]
  #[test]
  fn test_icon_svg_and_logo_replace_each_other() {
    let solid = Badge::new().icon_svg("<svg></svg>").logo("rust");
    assert_eq!(solid.icon_svg, None);
    assert_eq!(solid.logo, Some("rust".to_string()));

    let custom = Badge::new().logo("rust").icon_svg("<svg></svg>");
    assert_eq!(custom.logo, None);
    assert_eq!(custom.icon_svg, Some("<svg></svg>".to_string()));
  }

  #[cfg(not(feature = "simple-icons"))]
  #[test]
  fn test_logo_fields_are_ignored_without_simple_icons() {
    let badge: Badge =
      serde_json::from_str(r#"{"value":"Rust","logo":"rust","logoColor":"red","unknown":"value"}"#)
        .unwrap();
    let json = badge.to_json();
    let svg = badge.to_svg();

    assert!(!json.contains(r#""logo""#));
    assert!(!json.contains(r#""logoColor""#));
    assert!(!svg.contains("<image"));
  }

  #[cfg(feature = "simple-icons")]
  #[test]
  fn test_logo_is_rendered_with_simple_icons() {
    let default = Badge::new().value("Rust").logo("rust").to_svg();
    let red = Badge::new().value("Rust").logo("rust").logo_color(Color::Red).to_svg();

    assert!(default.contains(&get_icon("rust", &Color::White).unwrap()));
    assert!(red.contains(&get_icon("rust", &Color::Red).unwrap()));
  }

  #[cfg(feature = "simple-icons")]
  #[test]
  fn test_default_logo_matches_its_text_color() {
    let mono = Badge::new().value("Rust").value_color(Color::White).logo("rust").to_svg();
    assert!(mono.contains(&get_icon("rust", &Color::Black).unwrap()));
    assert!(mono.contains(r##"fill="#18181b""##));

    let split = Badge::new()
      .label("language")
      .label_color(Color::White)
      .value("Rust")
      .value_color(Color::Red)
      .logo("rust")
      .to_svg();
    assert!(split.contains(&get_icon("rust", &Color::Black).unwrap()));
  }

  #[cfg(feature = "simple-icons")]
  #[test]
  fn test_try_logo_validates_slug() {
    let badge = Badge::new().icon_svg("<svg></svg>").try_logo("rust").unwrap();
    assert_eq!(badge.logo, Some("rust".to_string()));
    assert_eq!(badge.icon_svg, None);

    let error = Badge::new().try_logo("not-a-real-simple-icon").unwrap_err();
    assert!(matches!(&error, Error::UnknownLogo(slug) if slug == "not-a-real-simple-icon"));
    assert_eq!(error.to_string(), "unknown Simple Icons slug 'not-a-real-simple-icon'");
  }

  #[cfg(feature = "simple-icons")]
  #[test]
  fn test_unknown_logo_is_omitted() {
    let svg = Badge::new().value("Rust").logo("not-a-real-simple-icon").to_svg();
    assert!(!svg.contains("<image"));
  }

  #[test]
  fn test_for_version() {
    // Test empty/unknown values
    let rs = Badge::new().for_version("pkg", "");
    assert_eq!(rs.value, Some("unknown".to_string()));
    assert_eq!(rs.label, Some("pkg".to_string()));

    // Test version with v prefix
    let rs = Badge::new().for_version("pkg", "v1.0.0");
    assert_eq!(rs.value, Some("v1.0.0".to_string()));
    assert_eq!(rs.value_color, Some(Color::Blue));

    // Test version without v prefix
    let rs = Badge::new().for_version("pkg", "1.0.0");
    assert_eq!(rs.value, Some("v1.0.0".to_string()));
    assert_eq!(rs.value_color, Some(Color::Blue));

    // Test version without v prefix
    let rs = Badge::new().for_version("pkg", "1.0.0");
    assert_eq!(rs.value, Some("v1.0.0".to_string()));
    assert_eq!(rs.value_color, Some(Color::Blue));

    // Test beta version
    let rs = Badge::new().for_version("pkg", "v1.0.0-beta");
    assert_eq!(rs.value_color, Some(Color::Cyan));

    // Test v0 version
    let rs = Badge::new().for_version("pkg", "v0.1.0");
    assert_eq!(rs.value_color, Some(Color::Orange));
  }

  #[test]
  fn test_for_stars_normalizes_invalid_and_out_of_range_scores() {
    for (value, max_value, expected, expected_color) in [
      (4.5, 5.0, "★★★★½", Color::Green),
      (10.0, 5.0, "★★★★★", Color::Green),
      (-1.0, 5.0, "☆☆☆☆☆", Color::Red),
      (0.0, 0.0, "☆☆☆☆☆", Color::Red),
      (f64::NAN, 5.0, "☆☆☆☆☆", Color::Red),
      (5.0, f64::INFINITY, "☆☆☆☆☆", Color::Red),
    ] {
      let badge = Badge::new().for_stars("rating", value, max_value);
      assert_eq!(badge.value.as_deref(), Some(expected));
      assert_eq!(badge.value_color, Some(expected_color));
    }
  }

  #[test]
  fn test_for_license_colors() {
    for license in ["MIT", "Apache-2.0", "Apache 2.0", "BSD-3-Clause"] {
      assert_eq!(Badge::new().for_license(license).value_color, Some(Color::Blue));
    }

    for license in ["GPL-3.0-or-later", "LGPLv3+", "MPL 2.0"] {
      assert_eq!(Badge::new().for_license(license).value_color, Some(Color::Orange));
    }

    for license in ["CC0-1.0", "Unlicense", "0BSD"] {
      assert_eq!(Badge::new().for_license(license).value_color, Some(Color::Lime));
    }

    for license in ["unknown", "NOASSERTION", ""] {
      assert_eq!(Badge::new().for_license(license).value_color, Some(Color::Gray));
    }
  }

  #[test]
  fn test_for_license_expressions() {
    assert_eq!(Badge::new().for_license("GPL-3.0-only OR MIT").value_color, Some(Color::Blue));
    assert_eq!(Badge::new().for_license("MIT AND CC0-1.0").value_color, Some(Color::Lime));
    assert_eq!(
      Badge::new().for_license("GPL-2.0-only WITH Classpath-exception-2.0").value_color,
      Some(Color::Orange)
    );
    assert_eq!(Badge::new().for_license("Apache 2.0 | GPLv3").value_color, Some(Color::Blue));
  }

  #[test]
  fn test_for_license_preserves_custom_color() {
    let solid = Badge::new().value_color(Color::Red).for_license("MIT");
    assert_eq!(solid.value_color, Some(Color::Red));

    let gradient = Badge::new().value_gradient([Color::Red, Color::Blue]).for_license("MIT");
    assert_eq!(gradient.value_color, None);
    assert_eq!(gradient.value_gradient, Some(vec![Color::Red, Color::Blue]));
  }

  #[cfg(feature = "axum")]
  #[test]
  fn test_axum_response_headers_can_be_overridden() {
    use axum::http::header;
    use axum::response::IntoResponse;

    let response = ([(header::CACHE_CONTROL, "no-store")], Badge::new()).into_response();

    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/svg+xml");
  }
}
