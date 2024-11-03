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

/// k8sfg - A CLI to display feature gates across Kubernetes versions.
///
/// ToDo
#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  /// The path to write the collected results to
  #[clap(short, long, default_value = "RESULTS.md")]
  pub path: PathBuf,

  #[clap(flatten)]
  pub verbose: Verbosity<InfoLevel>,
}

impl Cli {
  fn _collect(self) -> Result<()> {
    Ok(())
  }

  pub async fn write(self) -> Result<()> {
    let client = reqwest::Client::builder()
      .user_agent("k8sfg")
      .redirect(reqwest::redirect::Policy::limited(5))
      .build()?;

    let bin = crate::download_binary(client, "v1.31.0", "kubelet").await?;

    println!("{:?}", bin);

    Ok(())
  }
}
