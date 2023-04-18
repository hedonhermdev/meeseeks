use meeseeks::{llama_parser::LlamaParser, meeseeks_proto::master_agent_server};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use tonic::transport::Server;

use clap::Parser;
use meeseeks::master::MasterAgent;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct MasterCli {
    #[arg(short, long)]
    name: String,
    #[arg(short, long)]
    addr: SocketAddr,
    #[arg(short = 'm', long = "model-path")]
    llama_model_path: PathBuf,
}

impl MasterCli {
    pub async fn run() -> color_eyre::Result<()> {
        let args = MasterCli::parse();

        let mut master_addr = "http://".to_string();
        master_addr.push_str(&args.addr.to_string());

        let parser =
            LlamaParser::init(&args.llama_model_path).expect("failed to initialize llama parser");
        let master = Arc::new(MasterAgent::new(args.name, args.addr));
        let master_c = master.clone();

        let join = tokio::spawn(async move {
            tracing::info!("master is listening on address: {}", args.addr);
            match Server::builder()
                .add_service(master_agent_server::MasterAgentServer::new(master_c))
                .serve(args.addr)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    panic!("{}", e);
                }
            }
        });

        let stdin = std::io::stdin();

        let line = stdin.lines().next();

        let input =
"1. write a tweet about Elon Musk.
2. what is the capital of France?
3. write a short paragraph about Anne Hathaway.
4. what is 18 + 9?";

        let mut tasks = Vec::new();

        if parser.parse(input, &mut tasks).is_err() {
            color_eyre::eyre::bail!("failed to parse tasks");
        }

        let mut results = Vec::new();

        for task in tasks {
            let task_c = task.clone();
            let agent = match task.instruction.as_str() {
                "tweet" | "tweeit" => "meeseeks-tweetu",
                "search" => "meeseeks-wiki",
                "summary" => "meeseeks-wiki",
                "calculate" => "meeseeks-calculator",
                _ => "none",
            };

            if agent == "none" {
                tracing::info!(
                    "skipping task: {:?} as it does not match with any agent",
                    task
                );
                continue;
            }

            let res = master.send_task_to_agent(agent, task_c).await;

            tracing::info!("task: {:?}, res: {:?}", task, res);
            results.push(res);
        }

        println!("RESULTS: {:#?}", results);
        let _ = join.await?;

        Ok(())
    }
}
