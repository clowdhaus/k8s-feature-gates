pub mod cli;

use anyhow::{bail, Result};
pub use cli::Cli;
use once_cell::sync::Lazy;
use regex_lite::Regex;
use tokio::io::AsyncWriteExt;
use tracing::info;

// curl https://cdn.dl.k8s.io/release/stable-1.29.txt

pub async fn collect_feature_gates(client: reqwest::Client) -> Result<()> {
  let version = "v1.29.10";
  let binary = "kubelet";

  let bin = download_binary(client, version, binary).await?;
  let content = get_binary_output(&bin)?;
  let feature_gates = extract_feature_gates(&content)?;

  println!("Feature Gates: {:#?}", feature_gates);

  Ok(())
}

// MacOs is not supported !!!!
fn get_url(version: &str, binary: &str) -> Result<String> {
  let arch = std::env::consts::ARCH;
  let arch = match arch {
    "x86" | "x86_64" => "amd64",
    "arm" | "aarch64" | "loongarch64" => "arm64",
    _ => bail!("Unsupported architecture: {arch}"),
  };
  let os = std::env::consts::OS;
  let os = match os {
    "linux" | "freebsd" | "netbsd" | "openbsd" => "linux",
    "windows" => "windows",
    _ => bail!("Unsupported OS: {os}"),
  };

  Ok(format!("https://dl.k8s.io/{version}/bin/{os}/{arch}/{binary}"))
}

async fn download_binary(client: reqwest::Client, version: &str, binary: &str) -> Result<String> {
  let url = get_url(version, binary)?;
  info!("Download URL: {url}");
  let mut response = client.get(url).send().await?;

  let bin_dir = std::env::current_dir()?;
  let fname = bin_dir.join(binary);

  let mut dest = tokio::fs::OpenOptions::new()
    .truncate(true)
    .write(true)
    .create(true)
    .mode(0o755)
    .open(&fname)
    .await?;

  while let Some(chunk) = response.chunk().await? {
    dest.write_all(&chunk).await?;
  }
  dest.flush().await?;

  let bin = fname.to_string_lossy().to_string();
  Ok(bin)
}

fn get_binary_output(binary: &str) -> Result<String> {
  let proc = std::process::Command::new(binary)
    .args(vec!["--help"])
    .stdout(std::process::Stdio::piped())
    .stdin(std::process::Stdio::piped())
    .spawn()?;

  let result = proc.wait_with_output()?;
  info!("Result: {:#?}", result);
  let out = String::from_utf8_lossy(&result.stdout).to_string();

  Ok(out)
}

#[derive(Debug)]
enum FeatureLevel {
  Alpha,
  Beta,
  Ga,
}

impl std::fmt::Display for FeatureLevel {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      FeatureLevel::Alpha => write!(f, "ALPHA"),
      FeatureLevel::Beta => write!(f, "BETA"),
      FeatureLevel::Ga => write!(f, "GA"),
    }
  }
}

impl std::convert::From<&str> for FeatureLevel {
  fn from(s: &str) -> Self {
    match s {
      "ALPHA" => FeatureLevel::Alpha,
      "BETA" => FeatureLevel::Beta,
      "GA" => FeatureLevel::Ga,
      _ => panic!("Invalid feature level: {s}"),
    }
  }
}

#[allow(dead_code)]
#[derive(Debug)]
struct FeatureGate {
  name: String,
  level: FeatureLevel,
  default: bool,
}

fn extract_feature_gates(content: &str) -> Result<Vec<FeatureGate>> {
  static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(.*?)=.*?\((ALPHA|BETA|GA).*default=(true|false)").unwrap());

  let mut results = vec![];
  for (_, [name, level, default]) in RE.captures_iter(content).map(|c| c.extract()) {
    results.push(FeatureGate {
      name: name.trim().to_string(),
      level: level.into(),
      default: default.parse()?,
    })
  }

  Ok(results)
}
