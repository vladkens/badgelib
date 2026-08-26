use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rayon::prelude::*;

const SIMPLE_ICONS_REPOSITORY: &str = "https://github.com/simple-icons/simple-icons.git";
const SIMPLE_ICONS_BASELINE: &str = "14.0.0";

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct ReleaseVersion(u64, u64, u64);

impl ReleaseVersion {
  fn parse(tag: &str) -> Option<Self> {
    let mut parts = tag.strip_prefix('v').unwrap_or(tag).split('.');
    let version =
      Self(parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    parts.next().is_none().then_some(version)
  }
}

#[derive(Debug)]
struct ReleaseTag {
  name: String,
  version: ReleaseVersion,
  created_at: i64,
}

struct ReleaseSnapshot {
  name: String,
  icons: BTreeMap<String, String>,
}

struct ReleaseChange {
  version: String,
  total: usize,
  added: usize,
  removed: usize,
  changed: usize,
}

struct IconStats {
  baseline_icon_count: usize,
  releases: Vec<ReleaseChange>,
  latest_release: String,
  latest_icon_count: usize,
  historical_icon_count: usize,
  total_icon_count: usize,
  path_bytes: usize,
}

struct CollectedIcons {
  icons: BTreeMap<String, String>,
  baseline_icon_count: usize,
  releases: Vec<ReleaseChange>,
  latest_release: String,
  latest_icon_count: usize,
}

struct GeneratedFile {
  bytes: u64,
  changed: bool,
}

fn git_output(repo: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
  let output = Command::new("git")
    .arg("-C")
    .arg(repo)
    .args(args)
    .output()
    .map_err(|e| format!("failed to run git {}: {e}", args.join(" ")))?;

  if output.status.success() {
    Ok(output.stdout)
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("git {} failed: {stderr}", args.join(" ")))
  }
}

fn prepare_repository(url: &str, destination: &Path) -> Result<(), String> {
  if destination.exists() {
    git_output(destination, &["fetch", "--force", "--tags", "origin"])?;
    return Ok(());
  }

  let parent = destination.parent().ok_or_else(|| {
    format!("temporary Simple Icons repository has no parent: {}", destination.display())
  })?;
  fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
  let output = Command::new("git")
    .args(["clone", "--bare", "--no-single-branch", "--tags", url])
    .arg(destination)
    .output()
    .map_err(|e| format!("failed to clone Simple Icons: {e}"))?;

  if output.status.success() {
    Ok(())
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("failed to clone Simple Icons: {stderr}"))
  }
}

fn release_tags(repo: &Path) -> Result<Vec<ReleaseTag>, String> {
  let output = git_output(
    repo,
    &["for-each-ref", "--format=%(refname:short)\t%(creatordate:unix)", "refs/tags"],
  )?;
  let output = String::from_utf8(output).map_err(|e| format!("git returned invalid UTF-8: {e}"))?;
  let mut tags = Vec::new();

  for line in output.lines() {
    let Some((name, created_at)) = line.rsplit_once('\t') else {
      return Err(format!("malformed tag record from git: {line:?}"));
    };
    let Some(version) = ReleaseVersion::parse(name) else {
      continue;
    };
    let created_at =
      created_at.parse().map_err(|e| format!("invalid creation date for tag {name}: {e}"))?;
    tags.push(ReleaseTag { name: name.into(), version, created_at });
  }

  tags.sort_by(|a, b| {
    a.version.cmp(&b.version).then(a.created_at.cmp(&b.created_at)).then(a.name.cmp(&b.name))
  });
  Ok(tags)
}

fn supported_release_tags(repo: &Path) -> Result<Vec<ReleaseTag>, String> {
  let baseline = ReleaseVersion::parse(SIMPLE_ICONS_BASELINE)
    .expect("the Simple Icons baseline must be a semantic version");
  let tags =
    release_tags(repo)?.into_iter().filter(|tag| tag.version > baseline).collect::<Vec<_>>();
  if tags.is_empty() {
    return Err(format!("no Simple Icons releases found after baseline {SIMPLE_ICONS_BASELINE}"));
  }
  Ok(tags)
}

fn icon_oids(repo: &Path, reference: &str) -> Result<BTreeMap<String, String>, String> {
  let mut icons = BTreeMap::new();
  let output = git_output(repo, &["ls-tree", "-r", "-z", reference, "--", "icons"])?;
  for record in output.split(|byte| *byte == 0).filter(|record| !record.is_empty()) {
    let tab = record
      .iter()
      .position(|byte| *byte == b'\t')
      .ok_or_else(|| format!("malformed tree entry in {reference}"))?;
    let metadata = std::str::from_utf8(&record[..tab])
      .map_err(|e| format!("malformed tree metadata in {reference}: {e}"))?;
    let path = std::str::from_utf8(&record[tab + 1..])
      .map_err(|e| format!("malformed icon path in {reference}: {e}"))?;
    let mut metadata = metadata.split_ascii_whitespace();
    let _mode = metadata.next();
    let kind = metadata.next();
    let oid = metadata.next();

    if kind != Some("blob") {
      continue;
    }
    let Some(slug) = path.strip_prefix("icons/").and_then(|path| path.strip_suffix(".svg")) else {
      continue;
    };
    if slug.contains('/') {
      continue;
    }
    let oid = oid.ok_or_else(|| format!("missing blob id for {path} in {reference}"))?;
    icons.insert(slug.into(), oid.into());
  }

  Ok(icons)
}

fn read_blobs(repo: &Path, oids: &BTreeSet<String>) -> Result<BTreeMap<String, String>, String> {
  let mut child = Command::new("git")
    .arg("-C")
    .arg(repo)
    .args(["cat-file", "--batch"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("failed to run git cat-file --batch: {e}"))?;
  let mut stdin = child.stdin.take().ok_or("failed to open git cat-file stdin")?;
  let requests = oids.iter().map(String::as_str).collect::<Vec<_>>().join("\n") + "\n";
  let writer = std::thread::spawn(move || stdin.write_all(requests.as_bytes()));
  let output = child.wait_with_output().map_err(|e| format!("failed to read git objects: {e}"))?;
  writer
    .join()
    .map_err(|_| "git object writer thread panicked".to_string())?
    .map_err(|e| format!("failed to request git objects: {e}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(format!("git cat-file --batch failed: {stderr}"));
  }

  let mut blobs = BTreeMap::new();
  let mut cursor = 0;
  for oid in oids {
    let header_end = output.stdout[cursor..]
      .iter()
      .position(|byte| *byte == b'\n')
      .map(|offset| cursor + offset)
      .ok_or_else(|| format!("missing git object header for {oid}"))?;
    let header = std::str::from_utf8(&output.stdout[cursor..header_end])
      .map_err(|e| format!("invalid git object header for {oid}: {e}"))?;
    let fields = header.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.get(1) == Some(&"missing") {
      return Err(format!(
        "git object {oid} is unavailable; fetch the full Simple Icons tag history"
      ));
    }
    if fields.get(1) != Some(&"blob") || fields.len() != 3 {
      return Err(format!("unexpected git object header for {oid}: {header}"));
    }
    let size =
      fields[2].parse::<usize>().map_err(|e| format!("invalid git object size for {oid}: {e}"))?;
    let data_start = header_end + 1;
    let data_end = data_start
      .checked_add(size)
      .filter(|end| *end < output.stdout.len())
      .ok_or_else(|| format!("truncated git object {oid}"))?;
    if output.stdout[data_end] != b'\n' {
      return Err(format!("malformed git object terminator for {oid}"));
    }
    let data = String::from_utf8(output.stdout[data_start..data_end].to_vec())
      .map_err(|e| format!("icon blob {oid} is not UTF-8: {e}"))?;
    blobs.insert(oid.clone(), data);
    cursor = data_end + 1;
  }

  Ok(blobs)
}

fn extract_path_data(svg: &str) -> Result<String, String> {
  let path_start = svg.find("<path").ok_or("SVG has no path element")?;
  let path = &svg[path_start + "<path".len()..];
  let path_end = path.find('>').ok_or("SVG has an unterminated path element")?;
  let attributes = &path[..path_end];
  let bytes = attributes.as_bytes();
  let mut cursor = 0;

  while cursor < bytes.len() {
    while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/') {
      cursor += 1;
    }
    let name_start = cursor;
    while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'=' {
      cursor += 1;
    }
    let name = &attributes[name_start..cursor];
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
      cursor += 1;
    }
    if cursor >= bytes.len() || bytes[cursor] != b'=' {
      continue;
    }
    cursor += 1;
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
      cursor += 1;
    }
    let Some(&quote @ (b'\'' | b'"')) = bytes.get(cursor) else {
      return Err(format!("path attribute {name:?} is not quoted"));
    };
    cursor += 1;
    let value_start = cursor;
    while cursor < bytes.len() && bytes[cursor] != quote {
      cursor += 1;
    }
    if cursor >= bytes.len() {
      return Err(format!("path attribute {name:?} is unterminated"));
    }
    let value = &attributes[value_start..cursor];
    cursor += 1;
    if name == "d" {
      return Ok(value.into());
    }
  }

  Err("SVG path has no d attribute".into())
}

fn read_icon_paths(
  repo: &Path,
  snapshots: &[ReleaseSnapshot],
) -> Result<BTreeMap<String, String>, String> {
  let mut origins = BTreeMap::new();
  for snapshot in snapshots {
    for (slug, oid) in &snapshot.icons {
      origins.entry(oid.clone()).or_insert_with(|| (slug.clone(), snapshot.name.clone()));
    }
  }
  let oids = origins.keys().cloned().collect();
  let blobs = read_blobs(repo, &oids)?;
  let paths = origins
    .into_iter()
    .collect::<Vec<_>>()
    .into_par_iter()
    .map(|(oid, (slug, release))| {
      let svg = blobs.get(&oid).ok_or_else(|| format!("missing loaded blob {oid}"))?;
      let path = extract_path_data(svg)
        .map_err(|e| format!("failed to parse icon {slug:?} from release {release}: {e}"))?;
      Ok((oid, path))
    })
    .collect::<Result<Vec<_>, String>>()?;
  Ok(paths.into_iter().collect())
}

fn collect_icons(repo: &Path) -> Result<CollectedIcons, String> {
  let tags = supported_release_tags(repo)?;
  let mut snapshots = Vec::with_capacity(tags.len() + 1);
  snapshots.push(ReleaseSnapshot {
    name: format!("baseline {SIMPLE_ICONS_BASELINE}"),
    icons: icon_oids(repo, SIMPLE_ICONS_BASELINE)?,
  });
  for tag in &tags {
    snapshots.push(ReleaseSnapshot { name: tag.name.clone(), icons: icon_oids(repo, &tag.name)? });
  }
  if snapshots[0].icons.is_empty() {
    return Err("no icons found in badgelib's Simple Icons baseline".into());
  }

  let paths = read_icon_paths(repo, &snapshots)?;
  let baseline_icon_count = snapshots[0].icons.len();
  let mut icons = BTreeMap::new();
  for (slug, oid) in &snapshots[0].icons {
    let path = paths.get(oid).ok_or_else(|| format!("missing parsed icon blob {oid}"))?;
    icons.insert(slug.clone(), path.clone());
  }

  let mut releases = Vec::with_capacity(tags.len());
  let mut previous = &snapshots[0].icons;
  for snapshot in &snapshots[1..] {
    let added = snapshot.icons.keys().filter(|slug| !previous.contains_key(*slug)).count();
    let removed = previous.keys().filter(|slug| !snapshot.icons.contains_key(*slug)).count();
    let mut changed = 0;
    for (slug, oid) in &snapshot.icons {
      if let Some(previous_oid) = previous.get(slug)
        && paths.get(previous_oid) != paths.get(oid)
      {
        changed += 1;
      }
      let path = paths.get(oid).ok_or_else(|| format!("missing parsed icon blob {oid}"))?;
      icons.insert(slug.clone(), path.clone());
    }
    releases.push(ReleaseChange {
      version: snapshot.name.clone(),
      total: snapshot.icons.len(),
      added,
      removed,
      changed,
    });
    previous = &snapshot.icons;
  }

  let latest_release = releases
    .last()
    .ok_or("no supported Simple Icons releases found after badgelib's baseline")?
    .version
    .clone();
  Ok(CollectedIcons {
    icons,
    baseline_icon_count,
    releases,
    latest_release,
    latest_icon_count: previous.len(),
  })
}

fn generate_icons(repo: &Path) -> Result<(String, IconStats), String> {
  let CollectedIcons { icons, baseline_icon_count, releases, latest_release, latest_icon_count } =
    collect_icons(repo)?;
  let entries = icons
    .iter()
    .map(|(name, data)| format!("  \"{name}\" => r###\"{data}\"###,"))
    .collect::<Vec<_>>()
    .join("\n");
  let code = format!(
    "pub(crate) static ICONS: phf::Map<&'static str, &'static str> = phf::phf_map! {{\n\
     {entries}\n\
     }};"
  );
  let total_icon_count = icons.len();
  let stats = IconStats {
    baseline_icon_count,
    releases,
    latest_release,
    latest_icon_count,
    historical_icon_count: total_icon_count.saturating_sub(latest_icon_count),
    total_icon_count,
    path_bytes: icons.values().map(String::len).sum(),
  };
  Ok((code, stats))
}

fn write_generated(root: &Path, relative_path: &str, body: &str) -> Result<GeneratedFile, String> {
  let out_file = root.join(relative_path);
  let filename = out_file.file_name().ok_or("generated file has no filename")?.to_string_lossy();
  let temp_file = out_file.with_file_name(format!(".{filename}.tmp.rs"));
  let code = format!("// This file is generated by scripts/update-icons.rs\n{body}\n");
  fs::write(&temp_file, code)
    .map_err(|e| format!("failed to write {}: {e}", temp_file.display()))?;
  let generated =
    fs::read(&temp_file).map_err(|e| format!("failed to read {}: {e}", temp_file.display()))?;
  let changed = fs::read(&out_file).map_or(true, |existing| existing != generated);
  fs::rename(&temp_file, &out_file)
    .map_err(|e| format!("failed to replace {}: {e}", out_file.display()))?;
  Ok(GeneratedFile { bytes: generated.len() as u64, changed })
}

fn format_count(value: usize) -> String {
  let digits = value.to_string();
  let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
  for (index, digit) in digits.chars().enumerate() {
    if index > 0 && (digits.len() - index).is_multiple_of(3) {
      formatted.push(',');
    }
    formatted.push(digit);
  }
  formatted
}

fn format_bytes(bytes: u64) -> String {
  const KIB: f64 = 1024.0;
  const MIB: f64 = KIB * 1024.0;
  if bytes as f64 >= MIB {
    format!("{:.2} MiB", bytes as f64 / MIB)
  } else if bytes as f64 >= KIB {
    format!("{:.2} KiB", bytes as f64 / KIB)
  } else {
    format!("{bytes} B")
  }
}

fn print_icon_stats(stats: &IconStats, output: &GeneratedFile, elapsed: Duration) {
  let status = if output.changed { "updated" } else { "unchanged" };
  println!("\nSimple Icons releases");
  println!("  {:<10} {:>7} {:>7} {:>8} {:>8}", "version", "total", "added", "removed", "changed");
  println!(
    "  {:<10} {:>7} {:>7} {:>8} {:>8}",
    "baseline",
    format_count(stats.baseline_icon_count),
    "-",
    "-",
    "-"
  );
  for release in &stats.releases {
    println!(
      "  {:<10} {:>7} {:>7} {:>8} {:>8}",
      release.version,
      format_count(release.total),
      format!("+{}", format_count(release.added)),
      format!("-{}", format_count(release.removed)),
      format!("~{}", format_count(release.changed))
    );
  }

  println!("\nSimple Icons summary");
  println!(
    "  baseline         : Simple Icons {} ({} icons)",
    SIMPLE_ICONS_BASELINE,
    format_count(stats.baseline_icon_count)
  );
  println!(
    "  releases scanned : {} ({} -> {})",
    format_count(stats.releases.len()),
    stats.releases.first().expect("release changes must not be empty").version,
    stats.latest_release
  );
  println!(
    "  latest release   : {} ({} icons)",
    stats.latest_release,
    format_count(stats.latest_icon_count)
  );
  println!("  historical slugs : +{}", format_count(stats.historical_icon_count));
  println!("  embedded slugs   : {}", format_count(stats.total_icon_count));
  println!("  SVG path data    : {}", format_bytes(stats.path_bytes as u64));
  println!("  generated map    : {} ({status})", format_bytes(output.bytes));
  println!("  elapsed          : {:.2}s", elapsed.as_secs_f64());
}

fn run() -> Result<(), String> {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"));
  let icons_started = Instant::now();
  let repository = root.join("target/simple-icons.git");
  prepare_repository(SIMPLE_ICONS_REPOSITORY, &repository)?;
  let (icons, icon_stats) = generate_icons(&repository)?;
  let icon_output = write_generated(root, "src/_icons.rs", &icons)?;
  let icons_elapsed = icons_started.elapsed();
  print_icon_stats(&icon_stats, &icon_output, icons_elapsed);
  Ok(())
}

fn main() {
  if let Err(error) = run() {
    eprintln!("Simple Icons update failed: {error}");
    std::process::exit(1);
  }
}
