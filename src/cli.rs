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
  #[clap(flatten)]
  pub verbose: Verbosity<InfoLevel>,
}

impl Cli {
  pub async fn write(self, client: reqwest::Client) -> Result<()> {
    for versions in [vec![29, 30, 31]] {
      let mut table = crate::collect_feature_gates(client.clone(), versions.to_owned()).await?;

      table.with(tabled::settings::Style::markdown());

      let path = PathBuf::from("results").join(format!(
        "1.{}-1.{}.md",
        versions.first().unwrap(),
        versions.last().unwrap()
      ));
      tokio::fs::write(path, table.to_string()).await?;
    }
    Ok(())
  }
}
