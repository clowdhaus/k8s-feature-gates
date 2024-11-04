pub mod cli;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
pub use cli::Cli;
use once_cell::sync::Lazy;
use regex_lite::Regex;
use tabled::{builder::Builder, Table};
use tempfile::{tempdir, TempDir};
use tokio::io::AsyncWriteExt;
use tracing::info;

static K8S_BINARIES: &[&str] = &[
  "kube-apiserver",
  "kubelet",
  "kube-controller-manager",
  "kube-proxy",
  "kube-scheduler",
];

type KubernetesVersion = String;
type FeatureGates = BTreeMap<String, FeatureGate>;
type FeatureGateNames = BTreeSet<String>;
type FeatureGateData = BTreeMap<KubernetesVersion, FeatureGates>;

async fn collect_feature_gates(client: reqwest::Client, k8s_minor_versions: Vec<i32>) -> Result<Table> {
  let tmp = tempdir()?;
  let mut data: FeatureGateData = BTreeMap::new();
  let mut names: FeatureGateNames = BTreeSet::new();

  for minor_version in k8s_minor_versions {
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

  to_table(data, names)
}

/// Convert feature gate date into a table format for display
fn to_table(fg_data: FeatureGateData, fg_names: FeatureGateNames) -> Result<Table> {
  let k8s_versions = fg_data.keys().map(|v| v.to_string()).collect::<Vec<String>>();

  // Construct table header
  let mut headers = vec!["Feature Gate".to_string()];
  for k8s_version in &k8s_versions {
    headers.push(k8s_version.to_owned());
  }

  let mut builder = Builder::default();
  builder.push_record(headers);

  for name in fg_names {
    let mut row = vec![name.to_owned()];
    for k8s_version in &k8s_versions {
      let something = fg_data[k8s_version].get(&name);
      match something {
        Some(fg) => {
          row.push(format!("`{}`/`{}`", fg.level, fg.default));
        }
        None => {
          row.push("N/A".to_string());
        }
      }
    }
    builder.push_record(row);
  }

  Ok(builder.build())
}

/// Get the download URL for the Kubernetes binary
///
/// MacOs/Windows are not supported
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

/// Get the `--help` output (stdout) from the binary provided
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

/// Extract feature gates from the `--help` output of the Kubernetes binary
fn extract_feature_gates(content: &str) -> Result<(FeatureGates, FeatureGateNames)> {
  static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(.*?)=.*?\((ALPHA|BETA|GA).*default=(true|false)").unwrap());

  let mut gates: FeatureGates = BTreeMap::new();
  let mut names: FeatureGateNames = BTreeSet::new();
  for (_, [name, level, default]) in RE.captures_iter(content).map(|c| c.extract()) {
    // Starting in 1.31, it appears feature gate names are prepended with `kube:<name>`
    // We need to remove this to match with prior version feature gate names
    let name = name.trim().replace("kube:", "").to_string();
    names.insert(name.clone());

    gates.insert(
      name.clone(),
      FeatureGate {
        name,
        level: level.into(),
        default: default.parse()?,
      },
    );
  }

  Ok((gates, names))
}
