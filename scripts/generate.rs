use std::path::Path;
use std::{fs, io};

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
