use anyhow::Result;
use clap::Parser;
use k8sfg::Cli;
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  let client = reqwest::Client::builder()
    .user_agent(format!("k8sfg/{VERSION}"))
    .redirect(reqwest::redirect::Policy::limited(5))
    .build()?;

  cli.write(client).await.map_err(Into::into)
}
