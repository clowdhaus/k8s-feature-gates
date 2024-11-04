pub mod cli;

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
pub use cli::Cli;
use json_to_table::json_to_table;
use once_cell::sync::Lazy;
use regex_lite::Regex;
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};
use tokio::io::AsyncWriteExt;
use tracing::info;

static K8S_MINOR_VERSIONS: &[i32] = &[27, 28, 29, 30, 31];
static K8S_BINARIES: &[&str] = &[
  "kube-apiserver",
  "kubelet",
  "kube-controller-manager",
  "kube-proxy",
  "kube-scheduler",
];

type KubernetesVersion = String;
type FeatureGates = Vec<FeatureGate>;
type FeatureGateNames = HashSet<String>;
type FeatureGateData = HashMap<KubernetesVersion, FeatureGates>;

pub async fn display_feature_gates(client: reqwest::Client) -> Result<()> {
  let (fg_data, fg_names) = collect_feature_gates(client).await?;
  let results = merge_feature_gates(fg_data, fg_names)?;

  let json = serde_json::to_value(results)?;
  let table = json_to_table(&json).to_string();
  println!("{}", table);

  Ok(())
}

async fn collect_feature_gates(client: reqwest::Client) -> Result<(FeatureGateData, FeatureGateNames)> {
  let tmp = tempdir()?;
  let mut data: FeatureGateData = HashMap::new();
  let mut names: FeatureGateNames = HashSet::new();

  for minor_version in K8S_MINOR_VERSIONS {
    let full_version = client
      .get(format!("https://cdn.dl.k8s.io/release/stable-1.{minor_version}.txt"))
      .send()
      .await?
      .text()
      .await?;

    for binary in K8S_BINARIES {
      let bin = download_binary(&client, &tmp, &full_version, binary).await?;
      let content = get_binary_output(&bin)?;
      let (fgs, fg_names) = extract_feature_gates(&content)?;

      data.insert(full_version.to_owned(), fgs);
      names.extend(fg_names);
    }
  }

  Ok((data, names))
}

fn merge_feature_gates(fg_data: FeatureGateData, fg_names: FeatureGateNames) -> Result<Vec<Value>> {
  let mut results = vec![];

  for name in fg_names {
    for (k8s_version, feature_gates) in fg_data.iter() {
      for fg in feature_gates {
        results.push(json!({
          "Feature Gate Name": name,
          k8s_version: format!("{} - {}", fg.level, fg.default),
        }));
      }
    }
  }

  Ok(results)
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

async fn download_binary(client: &reqwest::Client, tmp: &TempDir, version: &str, binary: &str) -> Result<String> {
  let url = get_url(version, binary)?;
  info!("Download URL: {url}");
  let mut response = client.get(url).send().await?;

  let fname = tmp.path().join(binary);

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

fn extract_feature_gates(content: &str) -> Result<(FeatureGates, FeatureGateNames)> {
  static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(.*?)=.*?\((ALPHA|BETA|GA).*default=(true|false)").unwrap());

  let mut gates: Vec<_> = vec![];
  for (_, [name, level, default]) in RE.captures_iter(content).map(|c| c.extract()) {
    gates.push(FeatureGate {
      name: name.trim().to_string(),
      level: level.into(),
      default: default.parse()?,
    })
  }

  let names = gates.iter().map(|fg| fg.name.clone()).collect::<HashSet<String>>();

  Ok((gates, names))
}
