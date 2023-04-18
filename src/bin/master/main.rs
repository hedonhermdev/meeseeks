mod cli;

use cli::MasterCli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    MasterCli::run().await?;

    Ok(())
}
