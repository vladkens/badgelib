use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Color;
#[cfg(feature = "axum")]
use crate::param::Format;
use crate::param::{Period, Style};
use crate::utils::{
  cacl_width, empty_string_as_none, get_icon, millify, millify_iec, rating_color, text_color,
};

#[cfg(feature = "axum")]
fn default_cache() -> u32 {
  86400 // 24 hours
}

fn gradient_offset(index: usize, color_count: usize) -> String {
  let offset = index as f32 * 100.0 / (color_count - 1) as f32;
  format!("{offset:.3}%")
}

fn gradient_text_color(colors: &[Color]) -> Color {
  if colors.iter().all(|color| text_color(color) == Color::Black) {
    Color::Black
  } else {
    Color::White
  }
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

  #[serde(rename = "logo", alias = "icon")]
  logo: Option<String>,

  #[serde(rename = "logoColor", alias = "iconColor")]
  #[serde(deserialize_with = "empty_string_as_none", default)]
  logo_color: Option<Color>,

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

  /// Adds a logo by its [Simple Icons](https://simpleicons.org) slug.
  pub fn logo(mut self, logo: &str) -> Self {
    self.logo = Some(logo.into());
    self
  }

  /// Sets the logo color.
  pub fn logo_color(mut self, color: Color) -> Self {
    self.logo_color = Some(color);
    self
  }

  /// Sets the corner radius, clamped to `12` during rendering.
  pub fn radius(mut self, radius: u8) -> Self {
    self.radius = Some(radius);
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

  /// Configures a blue license badge.
  pub fn for_license(mut self, license: &str) -> Self {
    self.label = self.label.or(Some("license".into()));
    self.value = Some(license.into());
    if self.value_color.is_none() && self.value_gradient.is_none() {
      self.value_color = Some(Color::Blue);
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
      let scale = max_value / 5.0;
      let score = value / scale;

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
    let icon = get_icon(self.logo.as_deref().unwrap_or_default(), &self.logo_color);

    let ltext = self.label.clone().map(|x| x.trim().to_string()).unwrap_or_default();
    let rtext = self.value.clone().map(|x| x.trim().to_string()).unwrap_or_default();
    let (has_text, has_icon) = (!ltext.is_empty(), icon.is_some());

    #[allow(clippy::nonminimal_bool)]
    let mono = (!has_text && !has_icon)
      || (has_icon && !has_text && self.label_color.is_none() && self.label_gradient.is_none())
      || (ltext.is_empty() && rtext.is_empty());

    let fz = 110.0;
    let ltw = cacl_width(&ltext);
    let rtw = cacl_width(&rtext);
    let pad = fz * 0.5; // left / right padding
    let gap = pad / 1.5; // gap between left and right text

    let iw = if icon.is_some() { fz * 1.2 } else { 0.0 };
    #[allow(unused_assignments)]
    let (mut lx, mut lw, mut rx, mut rw) = (0.0, 0.0, 0.0, 0.0);

    if mono {
      rx = if has_icon { pad + iw + gap } else { pad };
      rw = if rtext.is_empty() { rx - gap + pad } else { rx + rtw + gap };
    } else {
      lx = if has_icon { pad + iw + gap } else { pad };
      lw = if has_text { lx + ltw + gap } else { lx };
      rx = lw + gap;
      rw = rx + rtw + pad - lw;
    }

    let (w, h) = (lw + rw, fz * 1.75);
    let y = (h + fz) / 2.0 - fz / 6.0;

    let title = if has_text { format!("{ltext}: {rtext}") } else { rtext.to_string() };
    let (outx, outy) = (fz * 0.075 / 2.0, fz * 0.075);

    let hh = 20.0 * 1.0;
    let ww = w * hh / h;

    let lb_color = self.label_color.clone().unwrap_or(Color::Black);
    let rb_color = self.value_color.clone().unwrap_or(Color::Blue);
    let lt_color = self
      .label_gradient
      .as_deref()
      .map(gradient_text_color)
      .unwrap_or_else(|| text_color(&lb_color))
      .to_css();
    let rt_color = self
      .value_gradient
      .as_deref()
      .map(gradient_text_color)
      .unwrap_or_else(|| text_color(&rb_color))
      .to_css();
    let lb_color = lb_color.to_css();
    let rb_color = rb_color.to_css();

    let radius = self.radius.unwrap_or(if self.style == Style::Flat { 3 } else { 0 }).min(12);
    let radius = (fz / 12.0) * radius as f32;

    let svg = maud::html!(svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label=(title)
      viewBox=(format!("0 0 {} {}", w, h)) width=(ww) height=(hh) text-rendering="geometricPrecision"
    {
      title { (title) }

      // background gradient
      @if self.style == Style::Flat {
        linearGradient id="s" x2="0" y2="100%" {
          stop offset="0" stop-opacity=".1" stop-color="#eee" {}
          stop offset="1" stop-opacity=".1" {}
        }
      }

      @if let Some(colors) = &self.label_gradient {
        linearGradient id="lg" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2=(w-rw) y2="0" {
          @for (index, color) in colors.iter().enumerate() {
            stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
          }
        }
      }

      @if let Some(colors) = &self.value_gradient {
        linearGradient id="vg" gradientUnits="userSpaceOnUse" x1=(w-rw) y1="0" x2=(w) y2="0" {
          @for (index, color) in colors.iter().enumerate() {
            stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
          }
        }
      }

      // border-radius
      mask id="r" { rect width=(w) height=(h) rx=(radius) fill="#fff" {} }

      g mask="url(#r)" {
        @if has_text || has_icon {
          rect x="0" y="0" width=(w) height=(h)
            fill=(if self.label_gradient.is_some() { "url(#lg)" } else { &lb_color }) {}
        }
        rect x=(w-rw) y="0" width=(rw) height=(h)
          fill=(if self.value_gradient.is_some() { "url(#vg)" } else { &rb_color }) rx=(0) {}
        rect x="0" y="0" width=(w) height=(h) fill="url(#s)" {}
      }

      @if icon.is_some() {
        image x=(pad) y=((h-iw)/2.0) width=(iw) height=(iw) href=(icon.unwrap()) {}
      }

      g font-family="DejaVu Sans,Verdana,Geneva,sans-serif" font-size=(fz) aria-hidden="true" {
        @if has_text {
          text textLength=(ltw) x=(lx+outx) y=(y+outy) fill="#000" opacity="0.25" { (&ltext) }
          text textLength=(ltw) x=(lx) y=(y) fill=(lt_color) { (&ltext) }
        }
        text textLength=(rtw) x=(rx+outx) y=(y+outy) fill="#000" opacity="0.25" { (&rtext) }
        text textLength=(rtw) x=(rx) y=(y) fill=(rt_color) { (&rtext) }
      }
    });

    svg.into_string()
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
  fn test_value_gradient() {
    let badge = Badge::new().label("build").value("passing").value_gradient([
      Color::Red,
      Color::Orange,
      Color::Cyan,
    ]);
    let svg = badge.to_svg();

    assert!(svg.contains(r#"id="vg""#));
    assert!(svg.contains(r##"offset="0.000%" stop-color="#ef4444""##));
    assert!(svg.contains(r##"offset="50.000%" stop-color="#f97316""##));
    assert!(svg.contains(r##"offset="100.000%" stop-color="#06b6d4""##));
    assert!(svg.contains(r#"fill="url(#vg)""#));
    assert_eq!(badge.value_color, None);
    assert!(badge.to_json().contains(r#""gradient":["ef4444","f97316","06b6d4"]"#));
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
}
