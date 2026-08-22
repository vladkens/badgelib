use std::path::Path;
use std::{fs, io};

use badgelib::{Animation, Badge, Color, Period, Style};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
  Colors,
  Styles,
  Gradients,
  ForTheBadgeGradients,
  SimpleIcons,
  CustomIcons,
  Animations,
  ForTheBadgeAnimations,
  BuiltIns,
}

impl Section {
  const ALL: [Self; 9] = [
    Self::Colors,
    Self::Styles,
    Self::Gradients,
    Self::ForTheBadgeGradients,
    Self::SimpleIcons,
    Self::CustomIcons,
    Self::Animations,
    Self::ForTheBadgeAnimations,
    Self::BuiltIns,
  ];

  fn name(self) -> &'static str {
    match self {
      Self::Colors => "colors",
      Self::Styles => "styles",
      Self::Gradients => "gradients",
      Self::ForTheBadgeGradients => "gradients-for-the-badge",
      Self::SimpleIcons => "simple-icons",
      Self::CustomIcons => "custom-icons",
      Self::Animations => "animations",
      Self::ForTheBadgeAnimations => "animations-for-the-badge",
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

impl Example {
  fn for_the_badge_section(&self) -> Option<Section> {
    let animation =
      matches!(self.filename, "anim-dual.svg" | "anim-shine.svg" | "anim-aurora-grad.svg");

    match self.section {
      Section::Gradients => Some(Section::ForTheBadgeGradients),
      Section::Animations if animation => Some(Section::ForTheBadgeAnimations),
      _ => None,
    }
  }
}

fn hex(value: &str) -> Color {
  Color::try_from(value).expect("example colors must be valid")
}

fn simple_icon(name: &str, color: &str) -> Badge {
  Badge::new().value(name).value_color(hex(color)).try_logo(name).expect("example logo must exist")
}

fn for_the_badge_filename(filename: &str) -> String {
  let stem = filename.strip_suffix(".svg").expect("example filename must end with .svg");
  format!("{stem}-for-the-badge.svg")
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
      .filter_map(|example| {
        let (filename, alt) = if example.section == section {
          (example.filename.into(), example.alt.into())
        } else if example.for_the_badge_section() == Some(section) {
          (for_the_badge_filename(example.filename), format!("{} (For the badge)", example.alt))
        } else {
          return None;
        };
        let align = if section == Section::Styles { " align=\"middle\"" } else { "" };
        Some(format!("  <img src=\"examples/{filename}\" alt=\"{alt}\"{align}>"))
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
      filename: "color-green.svg",
      alt: "Green",
      section: Section::Colors,
      badge: Badge::new().label("color").value("green").value_color(Color::Green),
    },
    Example {
      filename: "color-lime.svg",
      alt: "Lime",
      section: Section::Colors,
      badge: Badge::new().label("color").value("lime").value_color(Color::Lime),
    },
    Example {
      filename: "color-yellow.svg",
      alt: "Yellow",
      section: Section::Colors,
      badge: Badge::new().label("color").value("yellow").value_color(Color::Yellow),
    },
    Example {
      filename: "color-orange.svg",
      alt: "Orange",
      section: Section::Colors,
      badge: Badge::new().label("color").value("orange").value_color(Color::Orange),
    },
    Example {
      filename: "color-red.svg",
      alt: "Red",
      section: Section::Colors,
      badge: Badge::new().label("color").value("red").value_color(Color::Red),
    },
    Example {
      filename: "color-blue.svg",
      alt: "Blue",
      section: Section::Colors,
      badge: Badge::new().label("color").value("blue").value_color(Color::Blue),
    },
    Example {
      filename: "color-cyan.svg",
      alt: "Cyan",
      section: Section::Colors,
      badge: Badge::new().label("color").value("cyan").value_color(Color::Cyan),
    },
    Example {
      filename: "color-gray.svg",
      alt: "Gray",
      section: Section::Colors,
      badge: Badge::new().label("color").value("gray").value_color(Color::Gray),
    },
    Example {
      filename: "color-black.svg",
      alt: "Black",
      section: Section::Colors,
      badge: Badge::new().label("color").value("black").value_color(Color::Black),
    },
    Example {
      filename: "color-white.svg",
      alt: "White",
      section: Section::Colors,
      badge: Badge::new().label("color").value("white").value_color(Color::White),
    },
    Example {
      filename: "style-flat.svg",
      alt: "Flat",
      section: Section::Styles,
      badge: Badge::new().label("build").value("passing").value_color(Color::Green),
    },
    Example {
      filename: "style-flat-square.svg",
      alt: "Flat square",
      section: Section::Styles,
      badge: Badge::new()
        .label("build")
        .value("passing")
        .value_color(Color::Green)
        .style(Style::FlatSquare),
    },
    Example {
      filename: "style-for-the-badge.svg",
      alt: "For the badge",
      section: Section::Styles,
      badge: Badge::new()
        .label("build")
        .value("passing")
        .value_color(Color::Green)
        .style(Style::ForTheBadge),
    },
    Example {
      filename: "grad-value.svg",
      alt: "Value gradient",
      section: Section::Gradients,
      badge: Badge::new()
        .label("version")
        .value("v1.2.3")
        .value_gradient([hex("fb7185"), hex("f97316"), hex("facc15")])
        .logo("rust"),
    },
    Example {
      filename: "grad-dual.svg",
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
      filename: "grad-mono.svg",
      alt: "Mono gradient",
      section: Section::Gradients,
      badge: Badge::new()
        .value("gradient badge")
        .value_gradient([hex("f43f5e"), hex("a855f7"), hex("06b6d4")])
        .logo("rust"),
    },
    Example {
      filename: "icon-rust.svg",
      alt: "Rust",
      section: Section::SimpleIcons,
      badge: simple_icon("rust", "9a3412"),
    },
    Example {
      filename: "icon-python.svg",
      alt: "Python",
      section: Section::SimpleIcons,
      badge: simple_icon("python", "306998"),
    },
    Example {
      filename: "icon-typescript.svg",
      alt: "TypeScript",
      section: Section::SimpleIcons,
      badge: simple_icon("typescript", "235a97"),
    },
    Example {
      filename: "icon-go.svg",
      alt: "Go",
      section: Section::SimpleIcons,
      badge: simple_icon("go", "00758d"),
    },
    Example {
      filename: "icon-docker.svg",
      alt: "Docker",
      section: Section::SimpleIcons,
      badge: simple_icon("docker", "1d63c4"),
    },
    Example {
      filename: "icon-kubernetes.svg",
      alt: "Kubernetes",
      section: Section::SimpleIcons,
      badge: simple_icon("kubernetes", "3265c7"),
    },
    Example {
      filename: "icon-postgresql.svg",
      alt: "PostgreSQL",
      section: Section::SimpleIcons,
      badge: simple_icon("postgresql", "336791"),
    },
    Example {
      filename: "icon-redis.svg",
      alt: "Redis",
      section: Section::SimpleIcons,
      badge: simple_icon("redis", "a82f24"),
    },
    Example {
      filename: "icon-custom.svg",
      alt: "Custom icon",
      section: Section::CustomIcons,
      badge: Badge::new().value("custom icon").icon_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
          <path d="M12 3 L14 10 L21 12 L14 14 L12 21 L10 14 L3 12 L10 10 Z" fill="#facc15" />
        </svg>"##,
      ),
    },
    Example {
      filename: "anim-value.svg",
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
      filename: "anim-label.svg",
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
      filename: "anim-dual.svg",
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
      filename: "anim-shine.svg",
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
      filename: "anim-aurora.svg",
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
      filename: "anim-aurora-grad.svg",
      alt: "Aurora (gradient background)",
      section: Section::Animations,
      badge: Badge::new()
        .value("northern lights")
        .value_gradient([hex("0f172a"), hex("164e63"), hex("312e81")])
        .animation(Animation::Aurora),
    },
    Example {
      filename: "builtin-version.svg",
      alt: "Version",
      section: Section::BuiltIns,
      badge: Badge::new().for_version("version", "1.2.0"),
    },
    Example {
      filename: "builtin-license.svg",
      alt: "License",
      section: Section::BuiltIns,
      badge: Badge::new().for_license("MIT"),
    },
    Example {
      filename: "builtin-downloads.svg",
      alt: "Downloads",
      section: Section::BuiltIns,
      badge: Badge::new().for_downloads(Period::Month, 1_234_567),
    },
    Example {
      filename: "builtin-ci.svg",
      alt: "CI status",
      section: Section::BuiltIns,
      badge: Badge::new().for_ci_status("build", true),
    },
    Example {
      filename: "builtin-size.svg",
      alt: "Size",
      section: Section::BuiltIns,
      badge: Badge::new().for_size("size", 1_234_567),
    },
    Example {
      filename: "builtin-rating.svg",
      alt: "Rating",
      section: Section::BuiltIns,
      badge: Badge::new().for_rating("rating", 4.5, 5.0),
    },
  ];

  for example in &examples {
    let path = output_dir.join(example.filename);
    fs::write(&path, example.badge.to_svg())?;
    println!("generated {}", path.display());

    if example.for_the_badge_section().is_some() {
      let filename = for_the_badge_filename(example.filename);
      let path = output_dir.join(filename);
      let svg = example.badge.clone().style(Style::ForTheBadge).to_svg();
      fs::write(&path, svg)?;
      println!("generated {}", path.display());
    }
  }

  update_readme(root, &examples)?;
  println!("updated {}", root.join("readme.md").display());

  let json = Badge::new().label("version").value("v1.2.0").to_json();
  println!("to_json() -> {json}");

  Ok(())
}
