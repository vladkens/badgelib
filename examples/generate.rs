use std::fs;
use std::io;
use std::path::Path;

use badgelib::{Badge, Color};

fn hex(value: &str) -> Color {
  Color::try_from(value).expect("example colors must be valid")
}

fn main() -> io::Result<()> {
  let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
  fs::create_dir_all(&output_dir)?;

  for entry in fs::read_dir(&output_dir)? {
    let path = entry?.path();
    if path.extension().is_some_and(|extension| extension == "svg") {
      fs::remove_file(path)?;
    }
  }

  let badges = [
    (
      "value.svg",
      Badge::new()
        .label("version")
        .value("v1.2.3")
        .value_gradient([hex("fb7185"), hex("f97316"), hex("facc15")])
        .logo("rust"),
    ),
    (
      "dual.svg",
      Badge::new()
        .label("release")
        .label_gradient([hex("334155"), hex("7c3aed")])
        .value("stable")
        .value_gradient([hex("ec4899"), hex("f97316")])
        .logo("github"),
    ),
    (
      "mono.svg",
      Badge::new()
        .value("gradient badge")
        .value_gradient([hex("f43f5e"), hex("a855f7"), hex("06b6d4")])
        .logo("rust"),
    ),
  ];

  for (filename, badge) in badges {
    let path = output_dir.join(filename);
    fs::write(&path, badge.to_svg())?;
    println!("generated {}", path.display());
  }

  Ok(())
}
