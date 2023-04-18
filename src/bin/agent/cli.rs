use std::{net::SocketAddr, str::FromStr};

use clap::Parser;
use meeseeks::{
    agent::Agent,
    meeseeks_proto::agent_server,
    tool::{Calculator, Tool, Tweetu, Wiki},
};
use tonic::transport::Server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AgentCli {
    #[arg(short, long)]
    name: String,
    #[arg(short, long)]
    description: String,
    #[arg(short, long)]
    tool: String,
    #[arg(short, long)]
    addr: SocketAddr,
    #[arg(short, long)]
    master: String,
}

impl AgentCli {
    pub async fn run() -> color_eyre::Result<()> {
        let args = AgentCli::parse();

        let tool = tool_from_name(&args.tool)?;

        let mut agent_addr = "http://".to_string();
        agent_addr.push_str(&args.addr.to_string());
        let mut agent = Agent::new(args.name, args.description, agent_addr, tool);

        agent.connect_to_master(args.master).await?;

        Server::builder()
            .add_service(agent_server::AgentServer::new(agent))
            .serve(args.addr)
            .await?;

        Ok(())
    }
}

fn tool_from_name(name: &str) -> Result<Tool, color_eyre::eyre::Error> {
    match name {
        "tweetu" => Ok(Tool::Tweetu(Tweetu::new())),
        "calculator" => Ok(Tool::Calculator(Calculator)),
        "wiki" => Ok(Tool::Wiki(Wiki::new())),
        _ => Err(color_eyre::eyre::Error::msg(format!(
            "no tool named: {}",
            name
        ))),
    }
}
