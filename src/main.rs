mod config;
mod dto;
mod server;
mod services;

use crate::config::config::AppConfig;
use crate::server::server::run;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = AppConfig::load()?;

    if let Err(_) = run(&cfg).await {
        std::process::exit(1);
    }

    Ok(())
}
