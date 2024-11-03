pub mod cli;

use anyhow::{bail, Result};
pub use cli::Cli;
use tokio::io::AsyncWriteExt;
use tracing::info;

// curl https://cdn.dl.k8s.io/release/stable-1.29.txt

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
  // let tmp_dir = tempfile::Builder::new().prefix("k8sfg-").tempdir()?;
  let tmp_dir = std::env::current_dir()?;
  let mut response = client.get(url).send().await?;

  // let fname = tmp_dir.path().join(binary);
  let fname = tmp_dir.join(binary);
  info!("Binary location: {:?}", fname);

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

  let bin = fname.to_string_lossy().to_string();
  let out = std::process::Command::new(format!("./{bin}"))
    .arg("--help")
    .stdout(std::process::Stdio::piped())
    .output()?;

  let stdout = String::from_utf8(out.stdout)?;
  Ok(stdout)
}
