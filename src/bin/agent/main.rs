use cli::AgentCli;

mod cli;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    AgentCli::run().await?;

    Ok(())
}
