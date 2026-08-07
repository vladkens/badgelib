use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Color;
#[cfg(feature = "axum")]
use crate::param::Format;
use crate::param::{Animation, Period, Style};
use crate::utils::{
  cacl_width, empty_string_as_none, get_icon, millify, millify_iec, rating_color, text_color,
  to_icon_uri,
};

#[cfg(feature = "axum")]
fn default_cache() -> u32 {
  86400 // 24 hours
}

fn gradient_offset(index: usize, color_count: usize) -> String {
  let offset = index as f32 * 100.0 / (color_count - 1) as f32;
  format!("{offset:.3}%")
}

/// Repeats `colors` into two back-to-back periods that both start and end on
/// `colors[0]`, so animating a translation by exactly one period loops
/// seamlessly (see [`Badge::flow`] and its use in `to_svg`).
fn looping_gradient_stops(colors: &[Color]) -> Vec<Color> {
  let mut period = colors.to_vec();
  period.push(colors[0].clone());

  let mut seq = period.clone();
  seq.extend(period.into_iter().skip(1));
  seq
}

/// Width of one full loop of a flowing gradient, so that each color-to-color
/// transition gets roughly `box_width` of its own space to play out in,
/// instead of cramming every transition into a single `box_width` (which
/// looks increasingly squished as more colors are added).
fn flow_period(box_width: f32, color_count: usize) -> f32 {
  box_width * (color_count as f32 - 1.0).max(1.0)
}

/// Loop duration proportional to `flow_period`, so the gradient always
/// scrolls at the same speed regardless of how many colors it has to cover.
fn flow_duration(period: f32, box_width: f32) -> f32 {
  6.0 * period / box_width
}

fn rgb_to_hsl(color: &Color) -> (f32, f32, f32) {
  let hex = color.to_hex();
  let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.0;
  let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.0;
  let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.0;
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let delta = max - min;
  let lightness = (max + min) / 2.0;
  let saturation = if delta == 0.0 { 0.0 } else { delta / (1.0 - (2.0 * lightness - 1.0).abs()) };
  let hue = if delta == 0.0 {
    0.0
  } else if max == r {
    60.0 * ((g - b) / delta).rem_euclid(6.0)
  } else if max == g {
    60.0 * ((b - r) / delta + 2.0)
  } else {
    60.0 * ((r - g) / delta + 4.0)
  };
  (hue, saturation, lightness)
}

fn hsl_to_css(hue: f32, saturation: f32, lightness: f32) -> String {
  let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
  let h = hue.rem_euclid(360.0) / 60.0;
  let x = chroma * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
  let (r, g, b) = match h {
    h if h < 1.0 => (chroma, x, 0.0),
    h if h < 2.0 => (x, chroma, 0.0),
    h if h < 3.0 => (0.0, chroma, x),
    h if h < 4.0 => (0.0, x, chroma),
    h if h < 5.0 => (x, 0.0, chroma),
    _ => (chroma, 0.0, x),
  };
  let m = lightness - chroma / 2.0;
  let channel = |value: f32| ((value + m) * 255.0).round() as u8;
  format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
}

fn aurora_palette(colors: &[Color]) -> ([String; 3], f32) {
  let hsl = colors.iter().map(rgb_to_hsl).collect::<Vec<_>>();
  let (_, saturation, _) =
    hsl.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((170.0, 0.0, 0.5));
  let hue = if saturation < 0.12 {
    170.0
  } else {
    hsl.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0
  };
  let average_lightness =
    hsl.iter().map(|(_, _, lightness)| lightness).sum::<f32>() / hsl.len() as f32;
  let (lightness, opacity) =
    if average_lightness > 0.62 { ([0.36, 0.42, 0.38], 0.28) } else { ([0.60, 0.66, 0.62], 0.38) };
  let saturation = saturation.max(0.72);
  (
    [
      hsl_to_css(hue - 25.0, saturation, lightness[0]),
      hsl_to_css(hue + 25.0, saturation, lightness[1]),
      hsl_to_css(hue + 80.0, saturation, lightness[2]),
    ],
    opacity,
  )
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

  /// Custom icon markup set via [`Badge::icon_svg`], bypassing the Simple
  /// Icons lookup used by `logo`.
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
  pub fn logo(mut self, logo: &str) -> Self {
    self.logo = Some(logo.into());
    self.icon_svg = None;
    self
  }

  /// Sets the logo color.
  pub fn logo_color(mut self, color: Color) -> Self {
    self.logo_color = Some(color);
    self
  }

  /// Sets a custom icon from raw, already-colored SVG markup, bypassing the
  /// [Simple Icons](https://simpleicons.org) lookup used by [`Badge::logo`].
  /// Use this for icons that need multiple colors or shapes Simple Icons
  /// doesn't have. The caller is responsible for the markup; it is emitted
  /// as-is, so don't pass it untrusted input.
  pub fn icon_svg(mut self, svg: impl Into<String>) -> Self {
    self.icon_svg = Some(svg.into());
    self.logo = None;
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
    let flow_animated = self.animation == Some(Animation::Flow);
    let shine_animated = self.animation == Some(Animation::Shine);
    let aurora_animated = self.animation == Some(Animation::Aurora);
    let icon = self
      .icon_svg
      .as_deref()
      .map(to_icon_uri)
      .or_else(|| get_icon(self.logo.as_deref().unwrap_or_default(), &self.logo_color));

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

    let hh = 20.0;
    let ww = w * hh / h;

    let lb_color = self.label_color.clone().unwrap_or(Color::Black);
    let rb_color = self.value_color.clone().unwrap_or(Color::Blue);
    let aurora = if aurora_animated {
      let mut colors = self.label_gradient.clone().unwrap_or_else(|| vec![lb_color.clone()]);
      colors.extend(self.value_gradient.clone().unwrap_or_else(|| vec![rb_color.clone()]));
      Some(aurora_palette(&colors))
    } else {
      None
    };
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

    let bg_rects = maud::html!(
      @if has_text || has_icon {
        @if flow_animated && self.label_gradient.is_some() {
          rect class="flow-label" x="0" y="0" width=(w) height=(h) fill="url(#lg)" {}
        } @else {
          rect x="0" y="0" width=(w) height=(h)
            fill=(if self.label_gradient.is_some() { "url(#lg)" } else { &lb_color }) {}
        }
      }
      @if flow_animated && self.value_gradient.is_some() {
        rect class="flow-value" x=(w-rw) y="0" width=(rw) height=(h) fill="url(#vg)" rx=(0) {}
      } @else {
        rect x=(w-rw) y="0" width=(rw) height=(h)
          fill=(if self.value_gradient.is_some() { "url(#vg)" } else { &rb_color }) rx=(0) {}
      }
      rect x="0" y="0" width=(w) height=(h) fill="url(#s)" {}
      @if let Some((colors, opacity)) = &aurora {
        @let shapes = maud::html! {
          ellipse cx=(w*0.18) cy=(h*0.15) rx=(w*0.42) ry=(h*0.78)
            fill=(&colors[0]) fill-opacity=(opacity) {}
          ellipse cx=(w*0.58) cy=(h*0.78) rx=(w*0.38) ry=(h*0.72)
            fill=(&colors[1]) fill-opacity=(opacity*0.9) {}
          ellipse cx=(w*0.92) cy=(h*0.28) rx=(w*0.34) ry=(h*0.68)
            fill=(&colors[2]) fill-opacity=(opacity*0.8) {}
        };
        g class="aurora-static" filter="url(#aurora-blur)" { (shapes.clone()) }
        g class="aurora-motion" filter="url(#aurora-blur)" {
          ellipse cx=(w*0.18) cy=(h*0.15) rx=(w*0.42) ry=(h*0.78)
            fill=(&colors[0]) fill-opacity=(opacity) {
            (maud::PreEscaped(format!(
              r#"<animate attributeName="cx" values="{};{};{}" dur="11s" repeatCount="indefinite" />"#,
              w * 0.18, w * 0.82, w * 0.18
            )))
          }
          ellipse cx=(w*0.58) cy=(h*0.78) rx=(w*0.38) ry=(h*0.72)
            fill=(&colors[1]) fill-opacity=(opacity*0.9) {
            (maud::PreEscaped(format!(
              r#"<animate attributeName="cx" values="{};{};{}" dur="14s" repeatCount="indefinite" />"#,
              w * 0.72, w * 0.22, w * 0.72
            )))
          }
          ellipse cx=(w*0.92) cy=(h*0.28) rx=(w*0.34) ry=(h*0.68)
            fill=(&colors[2]) fill-opacity=(opacity*0.8) {
            (maud::PreEscaped(format!(
              r#"<animate attributeName="cx" values="{};{};{}" dur="17s" repeatCount="indefinite" />"#,
              w * 0.92, w * 0.38, w * 0.92
            )))
          }
        }
      }
      @if shine_animated {
        @let band = h * 1.15;
        rect class="shine" y="0" width=(band) height=(h) fill="url(#sh)" {
          (maud::PreEscaped(format!(
            r#"<animate attributeName="x" values="{};{};{};{}" keyTimes="0;0.25;0.70;1" calcMode="spline" keySplines="0 0 1 1;0.4 0 0.2 1;0 0 1 1" dur="4s" repeatCount="indefinite" />"#,
            -band,
            -band,
            w + band,
            w + band
          )))
        }
      }
    );

    let bg_group = maud::html!( g mask="url(#r)" { (bg_rects) } );

    let svg = maud::html!(svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label=(title)
      viewBox=(format!("0 0 {} {}", w, h)) width=(ww) height=(hh) text-rendering="geometricPrecision"
    {
      title { (title) }

      @if aurora_animated {
        style {
          (maud::PreEscaped(
            ".aurora-static{display:none}@media (prefers-reduced-motion: reduce) {.aurora-motion{display:none}.aurora-static{display:inline}}"
          ))
        }
      } @else if self.animation.is_some() {
        style {
          (maud::PreEscaped(
            "@media (prefers-reduced-motion: reduce) {.flow-label{fill:url(#lgs)}.flow-value{fill:url(#vgs)}.shine{display:none}}"
          ))
        }
      }

      // background gradient
      @if self.style == Style::Flat {
        linearGradient id="s" x2="0" y2="100%" {
          stop offset="0" stop-opacity=".1" stop-color="#eee" {}
          stop offset="1" stop-opacity=".1" {}
        }
      }

      @if let Some(colors) = &self.label_gradient {
        @if flow_animated {
          linearGradient id="lgs" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2=(w-rw) y2="0" {
            @for (index, color) in colors.iter().enumerate() {
              stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
            }
          }
          @let seq = looping_gradient_stops(colors);
          @let period = flow_period(w - rw, colors.len());
          @let dur = flow_duration(period, w - rw);
          linearGradient id="lg" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2=(period*2.0) y2="0" {
            @for (index, color) in seq.iter().enumerate() {
              stop offset=(gradient_offset(index, seq.len())) stop-color=(color.to_css()) {}
            }
            (maud::PreEscaped(format!(
              r#"<animateTransform attributeName="gradientTransform" type="translate" from="0 0" to="{} 0" dur="{:.2}s" repeatCount="indefinite" />"#,
              -period, dur
            )))
          }
        } @else {
          linearGradient id="lg" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2=(w-rw) y2="0" {
            @for (index, color) in colors.iter().enumerate() {
              stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
            }
          }
        }
      }

      @if let Some(colors) = &self.value_gradient {
        @if flow_animated {
          linearGradient id="vgs" gradientUnits="userSpaceOnUse" x1=(w-rw) y1="0" x2=(w) y2="0" {
            @for (index, color) in colors.iter().enumerate() {
              stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
            }
          }
          @let seq = looping_gradient_stops(colors);
          @let period = flow_period(rw, colors.len());
          @let dur = flow_duration(period, rw);
          linearGradient id="vg" gradientUnits="userSpaceOnUse" x1=(w-rw) y1="0" x2=(w-rw+period*2.0) y2="0" {
            @for (index, color) in seq.iter().enumerate() {
              stop offset=(gradient_offset(index, seq.len())) stop-color=(color.to_css()) {}
            }
            (maud::PreEscaped(format!(
              r#"<animateTransform attributeName="gradientTransform" type="translate" from="0 0" to="{} 0" dur="{:.2}s" repeatCount="indefinite" />"#,
              -period, dur
            )))
          }
        } @else {
          linearGradient id="vg" gradientUnits="userSpaceOnUse" x1=(w-rw) y1="0" x2=(w) y2="0" {
            @for (index, color) in colors.iter().enumerate() {
              stop offset=(gradient_offset(index, colors.len())) stop-color=(color.to_css()) {}
            }
          }
        }
      }

      @if shine_animated {
        linearGradient id="sh" x1="0" y1="0" x2="1" y2="0" gradientTransform="rotate(-12 .5 .5)" {
          stop offset="0%" stop-color="#fff" stop-opacity="0" {}
          stop offset="38%" stop-color="#fff" stop-opacity="0.05" {}
          stop offset="50%" stop-color="#fff" stop-opacity="0.3" {}
          stop offset="62%" stop-color="#fff" stop-opacity="0.05" {}
          stop offset="100%" stop-color="#fff" stop-opacity="0" {}
        }
      }

      @if aurora_animated {
        filter id="aurora-blur" x="-30%" y="-80%" width="160%" height="260%" {
          feGaussianBlur stdDeviation=(h*0.22) {}
        }
      }

      // border-radius
      mask id="r" { rect width=(w) height=(h) rx=(radius) fill="#fff" {} }

      (bg_group)

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
  fn test_icon_svg_bypasses_logo_lookup() {
    let raw = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M1 2" fill="#facc15" /></svg>"##;
    let badge = Badge::new().value("custom").icon_svg(raw);
    let svg = badge.to_svg();

    assert!(svg.contains(&to_icon_uri(raw)));
    assert_eq!(badge.logo, None);
  }

  #[test]
  fn test_icon_svg_and_logo_replace_each_other() {
    let solid = Badge::new().icon_svg("<svg></svg>").logo("rust");
    assert_eq!(solid.icon_svg, None);
    assert_eq!(solid.logo, Some("rust".to_string()));

    let custom = Badge::new().logo("rust").icon_svg("<svg></svg>");
    assert_eq!(custom.logo, None);
    assert_eq!(custom.icon_svg, Some("<svg></svg>".to_string()));
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
