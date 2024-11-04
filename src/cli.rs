use std::path::PathBuf;

use anstyle::{AnsiColor, Color, Style};
use anyhow::Result;
use clap::{builder::Styles, Parser};
use clap_verbosity_flag::{InfoLevel, Verbosity};

/// Styles for CLI
fn get_styles() -> Styles {
  Styles::styled()
    .header(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue))),
    )
    .literal(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
    .usage(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue))),
    )
    .placeholder(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Magenta))))
}

/// k8sfg - A CLI to discover the feature gates across Kubernetes versions.
#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  /// The path where the collected results are written
  #[clap(short, long, default_value = "RESULTS.md")]
  pub path: PathBuf,

  #[clap(flatten)]
  pub verbose: Verbosity<InfoLevel>,
}

impl Cli {
  pub async fn write(self, client: reqwest::Client) -> Result<()> {
    let mut table = self.collect(client).await?;

    table.with(tabled::settings::Style::markdown());
    tokio::fs::write(self.path, table.to_string()).await?;

    Ok(())
  }

  async fn collect(&self, client: reqwest::Client) -> Result<tabled::Table> {
    crate::collect_feature_gates(client).await
  }
}
