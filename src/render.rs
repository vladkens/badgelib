use std::fmt::{self, Write as _};

use super::Badge;
use crate::Color;
use crate::param::{Animation, Style};
use crate::utils::{cacl_width, text_color, to_icon_uri};
#[cfg(feature = "simple-icons")]
use crate::utils::{get_icon, has_icon};

struct Escaped<'a>(&'a str);

impl fmt::Display for Escaped<'_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for character in self.0.chars() {
      formatter.write_str(match character {
        '&' => "&amp;",
        '<' => "&lt;",
        '>' => "&gt;",
        '"' => "&quot;",
        _ => {
          formatter.write_char(character)?;
          continue;
        }
      })?;
    }
    Ok(())
  }
}

fn gradient_offset(index: usize, color_count: usize) -> String {
  let offset = index as f32 * 100.0 / (color_count - 1) as f32;
  format!("{offset:.3}%")
}

fn looping_gradient_stops(colors: &[Color]) -> Vec<Color> {
  let mut period = colors.to_vec();
  period.push(colors[0].clone());

  let mut seq = period.clone();
  seq.extend(period.into_iter().skip(1));
  seq
}

fn flow_period(box_width: f32, color_count: usize) -> f32 {
  box_width * (color_count as f32 - 1.0).max(1.0)
}

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

enum Fill {
  Solid(Color),
  Gradient(Vec<Color>),
}

impl Fill {
  fn text_color(&self) -> Color {
    match self {
      Self::Solid(color) => text_color(color),
      Self::Gradient(colors) => gradient_text_color(colors),
    }
  }

  fn css(&self, id: &str) -> String {
    match self {
      Self::Solid(color) => color.to_css(),
      Self::Gradient(_) => format!("url(#{id})"),
    }
  }

  fn colors(&self) -> Vec<Color> {
    match self {
      Self::Solid(color) => vec![color.clone()],
      Self::Gradient(colors) => colors.clone(),
    }
  }
}

struct Segment {
  x: f32,     // segment x
  w: f32,     // segment width
  tx: f32,    // text x
  tw: f32,    // text width
  fill: Fill, // background
}

struct Layout {
  w: f32,                 // badge width
  h: f32,                 // badge height
  fz: f32,                // font size
  y: f32,                 // text baseline
  ix: f32,                // icon x
  iw: f32,                // icon size
  label: Option<Segment>, // left segment
  value: Segment,         // right segment
}

struct Background {
  defs: String,
  fill: String,
  effect: String,
}

struct AuroraEllipse<'a> {
  x: f32,
  y: f32,
  rx: f32,
  ry: f32,
  color: &'a str,
  opacity: f32,
}

fn badge_fill(color: Option<&Color>, gradient: Option<&[Color]>, default: Color) -> Fill {
  match gradient {
    Some(colors) => Fill::Gradient(colors.to_vec()),
    None => Fill::Solid(color.cloned().unwrap_or(default)),
  }
}

fn has_badge_icon(badge: &Badge) -> bool {
  badge.icon_svg.is_some() || logo::exists(badge)
}

fn badge_icon(badge: &Badge, color: &Color) -> Option<String> {
  badge.icon_svg.as_deref().map(to_icon_uri).or_else(|| logo::render(badge, color))
}

fn standard_layout(badge: &Badge, ltext: &str, rtext: &str, has_icon: bool) -> Layout {
  let has_text = !ltext.is_empty();

  #[allow(clippy::nonminimal_bool)]
  let mono = (!has_text && !has_icon)
    || (has_icon && !has_text && badge.label_color.is_none() && badge.label_gradient.is_none())
    || (ltext.is_empty() && rtext.is_empty());

  let fz = 110.0;
  let ltw = cacl_width(ltext);
  let rtw = cacl_width(rtext);
  let pad = fz * 0.5; // left / right padding
  let gap = pad / 1.5; // gap between left and right text
  let iw = if has_icon { fz * 1.2 } else { 0.0 };

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
  let label = (lw > 0.0).then(|| Segment {
    x: 0.0,
    w: lw,
    tx: lx,
    tw: ltw,
    fill: badge_fill(badge.label_color.as_ref(), badge.label_gradient.as_deref(), Color::Black),
  });
  let value = Segment {
    x: w - rw,
    w: rw,
    tx: rx,
    tw: rtw,
    fill: badge_fill(badge.value_color.as_ref(), badge.value_gradient.as_deref(), Color::Blue),
  };

  Layout { w, h, fz, y: (h + fz) / 2.0 - fz / 6.0, ix: pad, iw, label, value }
}

fn for_the_badge_layout(badge: &Badge, ltext: &str, rtext: &str, has_icon: bool) -> Layout {
  let fz = 100.0;
  let pad = fz * 1.2;
  let icon_pad = pad * 0.75;
  let gap = pad * 0.5;
  let spacing = fz * 0.125;
  let iw = if has_icon { fz * 1.4 } else { 0.0 };

  let ltw = cacl_width(ltext) * fz / 110.0 + spacing * ltext.chars().count() as f32;
  let rtw = cacl_width(rtext) * fz / 110.0 + spacing * rtext.chars().count() as f32;

  let has_text = !ltext.is_empty();
  let lx = if has_icon { icon_pad + iw + gap } else { pad };

  let mut lw = 0.0;
  if has_text {
    lw = lx + ltw + pad;
  }

  let has_label_color = badge.label_color.is_some() || badge.label_gradient.is_some();
  if !has_text && has_icon && has_label_color {
    lw = icon_pad * 2.0 + iw;
  }

  let icon_gap = if rtext.is_empty() { gap - icon_pad } else { gap };
  let mut rx = lw + pad;
  let mut rw = pad * 2.0 + rtw;
  if lw == 0.0 && has_icon {
    rx = pad + iw + icon_gap;
    rw += iw + icon_gap;
  }

  let label = (lw > 0.0).then(|| Segment {
    x: 0.0,
    w: lw,
    tx: lx,
    tw: ltw,
    fill: badge_fill(badge.label_color.as_ref(), badge.label_gradient.as_deref(), Color::Black),
  });
  let value = Segment {
    x: lw,
    w: rw,
    tx: rx,
    tw: rtw,
    fill: badge_fill(badge.value_color.as_ref(), badge.value_gradient.as_deref(), Color::Blue),
  };

  let (w, h) = (lw + rw, fz * 2.8);
  Layout { w, h, fz, y: fz * 1.75, ix: icon_pad, iw, label, value }
}

fn render_gradient_stops(colors: &[Color]) -> String {
  colors
    .iter()
    .enumerate()
    .map(|(index, color)| {
      format!(
        r##"<stop offset="{}" stop-color="{}"></stop>"##,
        Escaped(&gradient_offset(index, colors.len())),
        Escaped(&color.to_css())
      )
    })
    .collect()
}

fn render_linear_gradient(id: &str, x1: f32, x2: f32, content: &str) -> String {
  format!(
    r##"<linearGradient id="{}" gradientUnits="userSpaceOnUse" x1="{}" y1="0" x2="{}" y2="0">{}</linearGradient>"##,
    Escaped(id),
    x1,
    x2,
    content
  )
}

fn render_gradient(id: &str, colors: &[Color], x: f32, w: f32, flow: bool) -> String {
  let x2 = x + w;
  let static_stops = render_gradient_stops(colors);
  if !flow {
    return render_linear_gradient(id, x, x2, &static_stops);
  }

  let static_id = format!("{id}s");
  let static_gradient = render_linear_gradient(&static_id, x, x2, &static_stops);
  let sequence = looping_gradient_stops(colors);
  let moving_stops = render_gradient_stops(&sequence);
  let period = flow_period(w, colors.len());
  let duration = flow_duration(period, w);
  let movement = format!(
    r##"<animateTransform attributeName="gradientTransform" type="translate" from="0 0" to="{} 0" dur="{:.2}s" repeatCount="indefinite"/>"##,
    -period, duration
  );
  let moving_content = format!("{moving_stops}{movement}");
  let moving_gradient = render_linear_gradient(id, x, x + period * 2.0, &moving_content);
  format!("{static_gradient}{moving_gradient}")
}

fn render_gradients(layout: &Layout, flow: bool) -> String {
  let label = layout.label.as_ref().map_or_else(String::new, |label| match &label.fill {
    Fill::Gradient(colors) => render_gradient("lg", colors, label.x, label.w, flow),
    Fill::Solid(_) => String::new(),
  });
  let value = match &layout.value.fill {
    Fill::Gradient(colors) => render_gradient("vg", colors, layout.value.x, layout.value.w, flow),
    Fill::Solid(_) => String::new(),
  };
  format!("{label}{value}")
}

fn render_fill(layout: &Layout, flow: bool) -> String {
  let label = if let Some(label) = &layout.label {
    let fill = label.fill.css("lg");
    if flow && matches!(&label.fill, Fill::Gradient(_)) {
      format!(
        r##"<rect class="flow-label" x="0" y="0" width="{}" height="{}" fill="url(#lg)"></rect>"##,
        layout.w, layout.h
      )
    } else {
      format!(
        r##"<rect x="0" y="0" width="{}" height="{}" fill="{}"></rect>"##,
        layout.w,
        layout.h,
        Escaped(&fill)
      )
    }
  } else {
    String::new()
  };

  let fill = layout.value.fill.css("vg");
  let value = if flow && matches!(&layout.value.fill, Fill::Gradient(_)) {
    format!(
      r##"<rect class="flow-value" x="{}" y="0" width="{}" height="{}" fill="url(#vg)" rx="0"></rect>"##,
      layout.value.x, layout.value.w, layout.h
    )
  } else {
    format!(
      r##"<rect x="{}" y="0" width="{}" height="{}" fill="{}" rx="0"></rect>"##,
      layout.value.x,
      layout.value.w,
      layout.h,
      Escaped(&fill)
    )
  };
  format!("{label}{value}")
}

fn render_static_background(layout: &Layout) -> Background {
  Background {
    defs: render_gradients(layout, false),
    fill: render_fill(layout, false),
    effect: String::new(),
  }
}

fn render_flow(layout: &Layout) -> Background {
  let gradients = render_gradients(layout, true);
  let reduced_motion = r##"<style>@media (prefers-reduced-motion: reduce) {.flow-label{fill:url(#lgs)}.flow-value{fill:url(#vgs)}}</style>"##;
  let defs = format!("{reduced_motion}{gradients}");
  Background { defs, fill: render_fill(layout, true), effect: String::new() }
}

fn render_shine(layout: &Layout) -> Background {
  let Background { defs, fill, .. } = render_static_background(layout);
  let band = layout.h * 1.15;
  let reduced_motion =
    r##"<style>@media (prefers-reduced-motion: reduce) {.shine{display:none}}</style>"##;
  let stops = [
    r##"<stop offset="0%" stop-color="#fff" stop-opacity="0"></stop>"##,
    r##"<stop offset="38%" stop-color="#fff" stop-opacity="0.05"></stop>"##,
    r##"<stop offset="50%" stop-color="#fff" stop-opacity="0.3"></stop>"##,
    r##"<stop offset="62%" stop-color="#fff" stop-opacity="0.05"></stop>"##,
    r##"<stop offset="100%" stop-color="#fff" stop-opacity="0"></stop>"##,
  ]
  .concat();
  let gradient = format!(
    r##"<linearGradient id="sh" x1="0" y1="0" x2="1" y2="0" gradientTransform="rotate(-12 .5 .5)">{}</linearGradient>"##,
    stops
  );
  let defs = format!("{defs}{reduced_motion}{gradient}");
  let movement = format!(
    r##"<animate attributeName="x" values="{0};{0};{1};{1}" keyTimes="0;0.25;0.70;1" calcMode="spline" keySplines="0 0 1 1;0.4 0 0.2 1;0 0 1 1" dur="4s" repeatCount="indefinite"/>"##,
    -band,
    layout.w + band
  );
  let effect = format!(
    r##"<rect class="shine" y="0" width="{}" height="{}" fill="url(#sh)">{}</rect>"##,
    band, layout.h, movement
  );

  Background { defs, fill, effect }
}

fn render_aurora_ellipse(ellipse: &AuroraEllipse<'_>, animation: &str) -> String {
  format!(
    r##"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" fill-opacity="{}">{}</ellipse>"##,
    ellipse.x,
    ellipse.y,
    ellipse.rx,
    ellipse.ry,
    Escaped(ellipse.color),
    ellipse.opacity,
    animation
  )
}

fn render_aurora_movement(start: f32, end: f32, duration: u8) -> String {
  format!(
    r##"<animate attributeName="cx" values="{0};{1};{0}" dur="{2}s" repeatCount="indefinite"/>"##,
    start, end, duration
  )
}

fn render_aurora_group(class: &str, ellipses: &str) -> String {
  format!(r##"<g class="{}" filter="url(#aurora-blur)">{}</g>"##, Escaped(class), ellipses)
}

fn render_aurora(layout: &Layout) -> Background {
  let Background { defs, fill, .. } = render_static_background(layout);
  let mut base = layout.label.as_ref().map(|label| label.fill.colors()).unwrap_or_default();
  base.extend(layout.value.fill.colors());
  let (palette, opacity) = aurora_palette(&base);
  let (w, h) = (layout.w, layout.h);
  let reduced_motion = r##"<style>.aurora-static{display:none}@media (prefers-reduced-motion: reduce) {.aurora-motion{display:none}.aurora-static{display:inline}}</style>"##;
  let blur = format!(r##"<feGaussianBlur stdDeviation="{}"></feGaussianBlur>"##, h * 0.22);
  let filter = format!(
    r##"<filter id="aurora-blur" x="-30%" y="-80%" width="160%" height="260%">{}</filter>"##,
    blur
  );
  let defs = format!("{defs}{reduced_motion}{filter}");

  let first = AuroraEllipse {
    x: w * 0.18,
    y: h * 0.15,
    rx: w * 0.42,
    ry: h * 0.78,
    color: &palette[0],
    opacity,
  };
  let second = AuroraEllipse {
    x: w * 0.58,
    y: h * 0.78,
    rx: w * 0.38,
    ry: h * 0.72,
    color: &palette[1],
    opacity: opacity * 0.9,
  };
  let third = AuroraEllipse {
    x: w * 0.92,
    y: h * 0.28,
    rx: w * 0.34,
    ry: h * 0.68,
    color: &palette[2],
    opacity: opacity * 0.8,
  };

  let static_ellipses = [
    render_aurora_ellipse(&first, ""),
    render_aurora_ellipse(&second, ""),
    render_aurora_ellipse(&third, ""),
  ]
  .concat();
  let static_group = render_aurora_group("aurora-static", &static_ellipses);

  let first_movement = render_aurora_movement(first.x, w * 0.82, 11);
  let second_movement = render_aurora_movement(w * 0.72, w * 0.22, 14);
  let third_movement = render_aurora_movement(third.x, w * 0.38, 17);
  let moving_ellipses = [
    render_aurora_ellipse(&first, &first_movement),
    render_aurora_ellipse(&second, &second_movement),
    render_aurora_ellipse(&third, &third_movement),
  ]
  .concat();
  let moving_group = render_aurora_group("aurora-motion", &moving_ellipses);
  let effect = format!("{static_group}{moving_group}");

  Background { defs, fill, effect }
}

fn render_background(animation: Option<Animation>, layout: &Layout) -> Background {
  match animation {
    Some(Animation::Flow) => render_flow(layout),
    Some(Animation::Shine) => render_shine(layout),
    Some(Animation::Aurora) => render_aurora(layout),
    None => render_static_background(layout),
  }
}

fn render_icon(icon: Option<&str>, layout: &Layout) -> String {
  match icon {
    Some(icon) => format!(
      r##"<image x="{}" y="{}" width="{}" height="{}" href="{}"></image>"##,
      layout.ix,
      (layout.h - layout.iw) / 2.0,
      layout.iw,
      layout.iw,
      Escaped(icon)
    ),
    None => String::new(),
  }
}

fn render_standard_text(
  segment: &Segment,
  baseline: f32,
  shadow_offset: (f32, f32),
  color: &str,
  text: &str,
) -> String {
  let shadow = format!(
    r##"<text textLength="{}" x="{}" y="{}" fill="#000" opacity="0.25">{}</text>"##,
    segment.tw,
    segment.tx + shadow_offset.0,
    baseline + shadow_offset.1,
    Escaped(text)
  );
  let foreground = format!(
    r##"<text textLength="{}" x="{}" y="{}" fill="{}">{}</text>"##,
    segment.tw,
    segment.tx,
    baseline,
    Escaped(color),
    Escaped(text)
  );
  format!("{shadow}{foreground}")
}

#[cfg(feature = "simple-icons")]
mod logo {
  use super::*;

  pub(super) fn exists(badge: &Badge) -> bool {
    badge.logo.as_deref().is_some_and(has_icon)
  }

  pub(super) fn render(badge: &Badge, default_color: &Color) -> Option<String> {
    get_icon(
      badge.logo.as_deref().unwrap_or_default(),
      badge.logo_color.as_ref().unwrap_or(default_color),
    )
  }
}

#[cfg(not(feature = "simple-icons"))]
mod logo {
  use super::*;

  pub(super) fn exists(_: &Badge) -> bool {
    false
  }

  pub(super) fn render(_: &Badge, _: &Color) -> Option<String> {
    None
  }
}

pub(super) fn svg(badge: &Badge) -> String {
  match badge.style {
    Style::ForTheBadge => render_for_the_badge_svg(badge),
    Style::Flat | Style::FlatSquare => render_standard_svg(badge),
  }
}

fn render_standard_svg(badge: &Badge) -> String {
  let has_icon = has_badge_icon(badge);

  let ltext = badge.label.clone().map(|x| x.trim().to_string()).unwrap_or_default();
  let rtext = badge.value.clone().map(|x| x.trim().to_string()).unwrap_or_default();
  let has_text = !ltext.is_empty();
  let layout = standard_layout(badge, &ltext, &rtext, has_icon);

  let lt_color = layout.label.as_ref().map(|label| label.fill.text_color());
  let rt_color = layout.value.fill.text_color();
  let icon_color = lt_color.as_ref().unwrap_or(&rt_color);
  let icon = badge_icon(badge, icon_color);
  let lt_color = lt_color.map(|color| color.to_css()).unwrap_or_default();
  let rt_color = rt_color.to_css();

  let title = if has_text { format!("{ltext}: {rtext}") } else { rtext.to_string() };
  let (outx, outy) = (layout.fz * 0.075 / 2.0, layout.fz * 0.075);
  let hh = 20.0;
  let ww = layout.w * hh / layout.h;

  let radius = badge.radius.unwrap_or(if badge.style == Style::Flat { 3 } else { 0 }).min(12);
  let radius = (layout.fz / 12.0) * radius as f32;
  let background = render_background(badge.animation, &layout);
  let Background { defs, fill, effect } = background;
  let highlight_defs = if badge.style == Style::Flat {
    let stops = [
      r##"<stop offset="0" stop-opacity=".1" stop-color="#eee"></stop>"##,
      r##"<stop offset="1" stop-opacity=".1"></stop>"##,
    ]
    .concat();
    format!(r##"<linearGradient id="s" x2="0" y2="100%">{}</linearGradient>"##, stops)
  } else {
    String::new()
  };
  let highlight = if badge.style == Style::Flat {
    format!(
      r##"<rect x="0" y="0" width="{}" height="{}" fill="url(#s)"></rect>"##,
      layout.w, layout.h
    )
  } else {
    String::new()
  };
  let icon = render_icon(icon.as_deref(), &layout);
  let label_text = if has_text {
    let label = layout.label.as_ref().unwrap();
    render_standard_text(label, layout.y, (outx, outy), &lt_color, &ltext)
  } else {
    String::new()
  };
  let value_text = render_standard_text(&layout.value, layout.y, (outx, outy), &rt_color, &rtext);

  let title_element = format!(r##"<title>{}</title>"##, Escaped(&title));
  let mask = format!(
    r##"<mask id="r"><rect width="{}" height="{}" rx="{}" fill="#fff"></rect></mask>"##,
    layout.w, layout.h, radius
  );
  let badge_body = format!(r##"<g mask="url(#r)">{}{}{}</g>"##, fill, highlight, effect);
  let text_group = format!(
    r##"<g font-family="DejaVu Sans,Verdana,Geneva,sans-serif" font-size="{}" aria-hidden="true">{}{}</g>"##,
    layout.fz, label_text, value_text
  );

  format!(
    r##"<svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="{title}" viewBox="0 0 {width} {height}" width="{rendered_width}" height="{rendered_height}" text-rendering="geometricPrecision">{title_element}{defs}{highlight_defs}{mask}{badge_body}{icon}{text_group}</svg>"##,
    title = Escaped(&title),
    width = layout.w,
    height = layout.h,
    rendered_width = ww,
    rendered_height = hh,
    title_element = title_element,
    defs = defs,
    highlight_defs = highlight_defs,
    mask = mask,
    badge_body = badge_body,
    icon = icon,
    text_group = text_group
  )
}

fn render_for_the_badge_svg(badge: &Badge) -> String {
  let has_icon = has_badge_icon(badge);

  let ltext = badge.label.clone().map(|text| text.trim().to_uppercase()).unwrap_or_default();
  let rtext = badge.value.clone().map(|text| text.trim().to_uppercase()).unwrap_or_default();
  let has_text = !ltext.is_empty();
  let layout = for_the_badge_layout(badge, &ltext, &rtext, has_icon);

  let lt_color = layout.label.as_ref().map(|label| label.fill.text_color());
  let rt_color = layout.value.fill.text_color();
  let icon_color = lt_color.as_ref().unwrap_or(&rt_color);
  let icon = badge_icon(badge, icon_color);
  let lt_color = lt_color.map(|color| color.to_css()).unwrap_or_default();
  let rt_color = rt_color.to_css();

  let title = if has_text { format!("{ltext}: {rtext}") } else { rtext.to_string() };
  let radius = badge.radius.unwrap_or(0).min(12);
  let radius = (layout.fz / 12.0) * radius as f32;
  let background = render_background(badge.animation, &layout);
  let Background { defs, fill, effect } = background;
  let icon = render_icon(icon.as_deref(), &layout);
  let label_text = if has_text {
    let label = layout.label.as_ref().unwrap();
    format!(
      r##"<text textLength="{}" x="{}" y="{}" fill="{}">{}</text>"##,
      label.tw,
      label.tx,
      layout.y,
      Escaped(&lt_color),
      Escaped(&ltext)
    )
  } else {
    String::new()
  };
  let value_text = format!(
    r##"<text textLength="{}" x="{}" y="{}" fill="{}" font-weight="bold">{}</text>"##,
    layout.value.tw,
    layout.value.tx,
    layout.y,
    Escaped(&rt_color),
    Escaped(&rtext)
  );

  let title_element = format!(r##"<title>{}</title>"##, Escaped(&title));
  let mask = format!(
    r##"<mask id="r"><rect width="{}" height="{}" rx="{}" fill="#fff"></rect></mask>"##,
    layout.w, layout.h, radius
  );
  let badge_body =
    format!(r##"<g mask="url(#r)" shape-rendering="crispEdges">{}{}</g>"##, fill, effect);
  let text_group = format!(
    r##"<g font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="{}" letter-spacing=".125em" aria-hidden="true">{}{}</g>"##,
    layout.fz, label_text, value_text
  );

  format!(
    r##"<svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="{title}" viewBox="0 0 {width} {height}" width="{rendered_width}" height="28" text-rendering="geometricPrecision">{title_element}{defs}{mask}{badge_body}{icon}{text_group}</svg>"##,
    title = Escaped(&title),
    width = layout.w,
    height = layout.h,
    rendered_width = layout.w / 10.0,
    title_element = title_element,
    defs = defs,
    mask = mask,
    badge_body = badge_body,
    icon = icon,
    text_group = text_group
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_for_the_badge_layout() {
    let svg = Badge::new().label("build").value("passing").style(Style::ForTheBadge).to_svg();

    assert!(svg.contains(r##"height="28""##));
    assert!(svg.contains(">BUILD</text>"));
    assert!(svg.contains(r##"font-weight="bold">PASSING</text>"##));
    assert!(svg.contains(r##"rx="0""##));
    assert!(!svg.contains(r##"id="s""##));
  }

  #[test]
  fn test_text_nodes_are_escaped_as_xml() {
    let svg = Badge::new().label(r##"&<>"'"##).value(r##"&<>"'"##).to_svg();
    let escaped = "&amp;&lt;&gt;&quot;'";

    assert!(svg.contains(&format!("<title>{escaped}: {escaped}</title>")));
    assert_eq!(svg.matches(&format!(">{escaped}</text>")).count(), 4);
  }

  #[test]
  fn test_double_quoted_attributes_are_escaped_as_xml() {
    let svg = Badge::new().label(r##"&<>"'"##).value(r##"&<>"'"##).to_svg();

    assert!(svg.contains(r##"aria-label="&amp;&lt;&gt;&quot;': &amp;&lt;&gt;&quot;'""##));
  }

  #[test]
  fn test_markup_shaped_text_cannot_inject_svg() {
    let injection = r##"</text><script>alert('x')</script><text data-x="pwned">"##;
    let escaped = r##"&lt;/text&gt;&lt;script&gt;alert('x')&lt;/script&gt;&lt;text data-x=&quot;pwned&quot;&gt;"##;
    let svg = Badge::new().label(injection).value("safe").to_svg();

    assert!(svg.contains(&format!("<title>{escaped}: safe</title>")));
    assert_eq!(svg.matches(&format!(">{escaped}</text>")).count(), 2);
    assert!(!svg.contains("<script>"));
    assert!(!svg.contains(r##"data-x="pwned""##));
  }

  #[test]
  fn test_for_the_badge_features() {
    let svg = Badge::new()
      .label("build")
      .value("flowing")
      .value_gradient([Color::Blue, Color::Cyan])
      .icon_svg(r##"<svg xmlns="http://www.w3.org/2000/svg"></svg>"##)
      .animation(Animation::Flow)
      .style(Style::ForTheBadge)
      .to_svg();

    assert!(svg.contains(r##"id="vg""##));
    assert!(svg.contains("<animateTransform"));
    assert!(svg.contains("<image "));
  }
}
