mod config;
mod dto;
mod repositories;
mod server;
mod services;
mod traits;
mod types;
mod utils;

use crate::config::config::AppConfig;
use crate::server::server::run;
use anyhow::Result;
use log::error;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = AppConfig::load()?;

    if let Err(e) = run(&cfg).await {
        error!("Ошибка: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
