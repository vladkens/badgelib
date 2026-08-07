use std::fs;
use std::io;
use std::path::Path;

use badgelib::{Animation, Badge, Color, Period};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
  Gradients,
  CustomIcons,
  Animations,
  BuiltIns,
}

impl Section {
  const ALL: [Self; 4] = [Self::Gradients, Self::CustomIcons, Self::Animations, Self::BuiltIns];

  fn name(self) -> &'static str {
    match self {
      Self::Gradients => "gradients",
      Self::CustomIcons => "custom-icons",
      Self::Animations => "animations",
      Self::BuiltIns => "built-ins",
    }
  }
}

struct Example {
  filename: &'static str,
  alt: &'static str,
  section: Section,
  badge: Badge,
}

fn vibe_icon(body: &str) -> String {
  format!(
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
      <style>
        .pulse, .spin, .bob, .twinkle {{ transform-box: fill-box; transform-origin: center; }}
        .pulse {{ animation: pulse 1.8s ease-in-out infinite; }}
        .spin {{ animation: spin 5s linear infinite; }}
        .bob {{ animation: bob 1.8s ease-in-out infinite; }}
        .twinkle {{ animation: twinkle 1.6s ease-in-out infinite alternate; }}
        .late {{ animation-delay: -.8s; }}
        @keyframes pulse {{ 50% {{ transform: scale(1.14) rotate(5deg); }} }}
        @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
        @keyframes bob {{ 50% {{ transform: translateY(-2px); }} }}
        @keyframes twinkle {{ from {{ transform: scale(.68) rotate(-10deg); opacity: .55; }} to {{ transform: scale(1.08) rotate(10deg); opacity: 1; }} }}
        @media (prefers-reduced-motion: reduce) {{ .pulse, .spin, .bob, .twinkle {{ animation: none; opacity: 1; }} }}
      </style>
      {body}
    </svg>"##
  )
}

fn vibe_badge(icon: &str) -> Badge {
  Badge::new()
    .value("Vibe Coded")
    .value_gradient([hex("581c87"), hex("1e3a8a"), hex("164e63")])
    .icon_svg(vibe_icon(icon))
    .animation(Animation::Aurora)
}

fn hex(value: &str) -> Color {
  Color::try_from(value).expect("example colors must be valid")
}

fn update_readme(root: &Path, examples: &[Example]) -> io::Result<()> {
  let path = root.join("readme.md");
  let mut readme = fs::read_to_string(&path)?;

  for section in Section::ALL {
    let name = section.name();
    let start = format!("<!-- examples:{name}:start -->");
    let end = format!("<!-- examples:{name}:end -->");
    let starts = readme.match_indices(&start).map(|(index, _)| index).collect::<Vec<_>>();
    let ends = readme.match_indices(&end).map(|(index, _)| index).collect::<Vec<_>>();

    if starts.len() != 1 || ends.len() != 1 || starts[0] >= ends[0] {
      return Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("readme.md must contain exactly one ordered {start} / {end} pair"),
      ));
    }

    let images = examples
      .iter()
      .filter(|example| example.section == section)
      .map(|example| {
        format!("  <img src=\"examples/{}\" alt=\"{}\">", example.filename, example.alt)
      })
      .collect::<Vec<_>>();
    if images.is_empty() {
      return Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("no examples declared for README section {name}"),
      ));
    }

    let replacement = format!("{start}\n<p align=\"center\">\n{}\n</p>\n{end}", images.join("\n"));
    readme.replace_range(starts[0]..ends[0] + end.len(), &replacement);
  }

  fs::write(path, readme)
}

fn main() -> io::Result<()> {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"));
  let output_dir = root.join("examples");
  fs::create_dir_all(&output_dir)?;

  for entry in fs::read_dir(&output_dir)? {
    let path = entry?.path();
    if path.extension().is_some_and(|extension| extension == "svg") {
      fs::remove_file(path)?;
    }
  }

  let examples = [
    Example {
      filename: "value.svg",
      alt: "Value gradient",
      section: Section::Gradients,
      badge: Badge::new()
        .label("version")
        .value("v1.2.3")
        .value_gradient([hex("fb7185"), hex("f97316"), hex("facc15")])
        .logo("rust"),
    },
    Example {
      filename: "dual.svg",
      alt: "Dual gradient",
      section: Section::Gradients,
      badge: Badge::new()
        .label("release")
        .label_gradient([hex("334155"), hex("7c3aed")])
        .value("stable")
        .value_gradient([hex("ec4899"), hex("f97316")])
        .logo("github"),
    },
    Example {
      filename: "mono.svg",
      alt: "Mono gradient",
      section: Section::Gradients,
      badge: Badge::new()
        .value("gradient badge")
        .value_gradient([hex("f43f5e"), hex("a855f7"), hex("06b6d4")])
        .logo("rust"),
    },
    Example {
      filename: "custom-icon.svg",
      alt: "Custom icon",
      section: Section::CustomIcons,
      badge: Badge::new().value("custom icon").icon_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
          <path d="M12 3 L14 10 L21 12 L14 14 L12 21 L10 14 L3 12 L10 10 Z" fill="#facc15" />
        </svg>"##,
      ),
    },
    Example {
      filename: "value-gradient-animated.svg",
      alt: "Flow (value side)",
      section: Section::Animations,
      badge: Badge::new()
        .label("build")
        .value("flowing")
        .value_gradient([hex("a855f7"), hex("3b82f6"), hex("ec4899"), hex("06b6d4")])
        .animation(Animation::Flow)
        .logo("rust"),
    },
    Example {
      filename: "label-gradient-animated.svg",
      alt: "Flow (label side)",
      section: Section::Animations,
      badge: Badge::new()
        .label("flowing")
        .value("build")
        .label_gradient([hex("a855f7"), hex("3b82f6"), hex("ec4899"), hex("06b6d4")])
        .value_color(hex("18181b"))
        .animation(Animation::Flow)
        .logo("rust"),
    },
    Example {
      filename: "dual-gradient-animated.svg",
      alt: "Flow (both sides)",
      section: Section::Animations,
      badge: Badge::new()
        .label("release")
        .label_gradient([hex("334155"), hex("7c3aed")])
        .value("stable")
        .value_gradient([hex("ec4899"), hex("f97316")])
        .animation(Animation::Flow)
        .logo("github"),
    },
    Example {
      filename: "shine-animated.svg",
      alt: "Shine",
      section: Section::Animations,
      badge: Badge::new()
        .label("build")
        .value("passing")
        .value_color(hex("18181b"))
        .animation(Animation::Shine)
        .logo("rust"),
    },
    Example {
      filename: "aurora-color.svg",
      alt: "Aurora (solid background)",
      section: Section::Animations,
      badge: Badge::new()
        .label("nightly")
        .value("aurora")
        .value_color(hex("1e3a8a"))
        .animation(Animation::Aurora)
        .logo("rust"),
    },
    Example {
      filename: "aurora-gradient.svg",
      alt: "Aurora (gradient background)",
      section: Section::Animations,
      badge: Badge::new()
        .value("northern lights")
        .value_gradient([hex("0f172a"), hex("164e63"), hex("312e81")])
        .animation(Animation::Aurora),
    },
    Example {
      filename: "vibe-coded-01-sparkles.svg",
      alt: "Vibe Coded: sparkles",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<path class="twinkle" d="M8 1.5 10 9l7.5 2-7.5 2L8 20.5 6 13l-5.5-2L6 9Z" fill="#f472b6"/><path class="twinkle late" d="m18 3 1.2 3.8L23 8l-3.8 1.2L18 13l-1.2-3.8L13 8l3.8-1.2Z" fill="#22d3ee"/><circle cx="19" cy="18.5" r="3" fill="#facc15"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-02-wand.svg",
      alt: "Vibe Coded: magic wand",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="bob"><path d="m5 20 11-11 3 3L8 23Z" fill="#22d3ee"/><path d="m15.2 9.8 2.6-2.6 3 3-2.6 2.6Z" fill="#f472b6"/></g><path class="twinkle" d="m7 1 1.3 3.7L12 6 8.3 7.3 7 11 5.7 7.3 2 6l3.7-1.3Z" fill="#facc15"/><path class="twinkle late" d="m18 1 .7 2.3L21 4l-2.3.7L18 7l-.7-2.3L15 4l2.3-.7Z" fill="#a78bfa"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-03-rainbow.svg",
      alt: "Vibe Coded: rainbow",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="pulse" fill="none" stroke-linecap="round" stroke-width="3"><path d="M3 18a9 9 0 0 1 18 0" stroke="#f472b6"/><path d="M6 18a6 6 0 0 1 12 0" stroke="#facc15"/><path d="M9 18a3 3 0 0 1 6 0" stroke="#22d3ee"/></g><circle cx="3" cy="19" r="2" fill="#e0f2fe"/><circle cx="21" cy="19" r="2" fill="#e0f2fe"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-04-comet.svg",
      alt: "Vibe Coded: comet",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="bob"><path d="M2 18 14 6l4 4L6 22Z" fill="#22d3ee" opacity=".65"/><path d="m2 12 10-6 6 6-10 6Z" fill="#a78bfa" opacity=".75"/><circle cx="17" cy="7" r="5" fill="#f472b6"/><circle cx="18.5" cy="5.5" r="1.7" fill="#facc15"/></g>"##,
      ),
    },
    Example {
      filename: "vibe-coded-05-heart.svg",
      alt: "Vibe Coded: pixel heart",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="pulse"><path d="M2 5h7v3h6V5h7v11h-3v3h-3v3H8v-3H5v-3H2Z" fill="#f472b6"/><path d="M5 8h4v3H5Zm10 0h4v3h-4Z" fill="#facc15"/><path d="M9 14h6v5H9Z" fill="#a78bfa"/></g>"##,
      ),
    },
    Example {
      filename: "vibe-coded-06-flower.svg",
      alt: "Vibe Coded: flower",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="spin"><ellipse cx="12" cy="5" rx="4" ry="5" fill="#f472b6"/><ellipse cx="12" cy="19" rx="4" ry="5" fill="#22d3ee"/><ellipse cx="5" cy="12" rx="5" ry="4" fill="#a78bfa"/><ellipse cx="19" cy="12" rx="5" ry="4" fill="#facc15"/></g><circle class="pulse" cx="12" cy="12" r="4.5" fill="#fb7185"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-07-orbit.svg",
      alt: "Vibe Coded: orbit",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="spin"><ellipse cx="12" cy="12" rx="10" ry="4.5" fill="none" stroke="#22d3ee" stroke-width="2"/><circle cx="22" cy="12" r="2.5" fill="#facc15"/><ellipse cx="12" cy="12" rx="4.5" ry="10" fill="none" stroke="#f472b6" stroke-width="2"/><circle cx="12" cy="2" r="2" fill="#a78bfa"/></g><circle class="pulse" cx="12" cy="12" r="4" fill="#fb7185"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-08-bolt.svg",
      alt: "Vibe Coded: lightning",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<path class="pulse" d="M13 1 4 14h7l-1 9 10-14h-7Z" fill="#facc15" stroke="#f472b6" stroke-width="1.5" stroke-linejoin="round"/><circle class="twinkle" cx="4" cy="5" r="2.5" fill="#22d3ee"/><circle class="twinkle late" cx="20" cy="19" r="2" fill="#a78bfa"/>"##,
      ),
    },
    Example {
      filename: "vibe-coded-09-prism.svg",
      alt: "Vibe Coded: prism",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="bob"><path d="m12 2 10 19H2Z" fill="#a78bfa" stroke="#e0f2fe" stroke-width="1.5" stroke-linejoin="round"/><path d="m12 2 2.7 13.5L22 21Z" fill="#22d3ee" opacity=".75"/><path d="m2 21 9-8 3.7 2.5L22 21Z" fill="#f472b6" opacity=".8"/><path d="m5 15 6-2 3.7 2.5L20 14" fill="none" stroke="#facc15" stroke-width="2"/></g>"##,
      ),
    },
    Example {
      filename: "vibe-coded-10-code.svg",
      alt: "Vibe Coded: playful code",
      section: Section::Animations,
      badge: vibe_badge(
        r##"<g class="pulse" fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="3.2"><path d="m8 5-6 7 6 7" stroke="#f472b6"/><path d="m16 5 6 7-6 7" stroke="#22d3ee"/><path d="m14 3-4 18" stroke="#facc15"/></g>"##,
      ),
    },
    Example {
      filename: "version.svg",
      alt: "Version",
      section: Section::BuiltIns,
      badge: Badge::new().for_version("version", "1.2.0"),
    },
    Example {
      filename: "license.svg",
      alt: "License",
      section: Section::BuiltIns,
      badge: Badge::new().for_license("MIT"),
    },
    Example {
      filename: "downloads.svg",
      alt: "Downloads",
      section: Section::BuiltIns,
      badge: Badge::new().for_downloads(Period::Month, 1_234_567),
    },
    Example {
      filename: "ci-status.svg",
      alt: "CI status",
      section: Section::BuiltIns,
      badge: Badge::new().for_ci_status("build", true),
    },
    Example {
      filename: "size.svg",
      alt: "Size",
      section: Section::BuiltIns,
      badge: Badge::new().for_size("size", 1_234_567),
    },
    Example {
      filename: "rating.svg",
      alt: "Rating",
      section: Section::BuiltIns,
      badge: Badge::new().for_rating("rating", 4.5, 5.0),
    },
  ];

  for example in &examples {
    let path = output_dir.join(example.filename);
    fs::write(&path, example.badge.to_svg())?;
    println!("generated {}", path.display());
  }

  update_readme(root, &examples)?;
  println!("updated {}", root.join("readme.md").display());

  let json = Badge::new().label("version").value("v1.2.0").to_json();
  println!("to_json() -> {json}");

  Ok(())
}
