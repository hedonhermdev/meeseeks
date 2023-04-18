use meeseeks::{llama_parser::LlamaParser, meeseeks_proto::{master_agent_server, Status}};
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
    listen: SocketAddr,
    #[arg(short, long)]
    addr: String,
    #[arg(short = 'm', long = "model-path")]
    llama_model_path: PathBuf,
}

impl MasterCli {
    pub async fn run() -> color_eyre::Result<()> {
        let args = MasterCli::parse();

        let mut master_addr = args.addr;

        let master = Arc::new(MasterAgent::new(args.name, args.listen));
        let master_c = master.clone();

        let join = tokio::spawn(async move {
            tracing::info!("master is listening on address: {}", args.listen);
            match Server::builder()
                .add_service(master_agent_server::MasterAgentServer::new(master_c))
                .serve(args.listen)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    panic!("{}", e);
                }
            }
        });

        let parser =
            LlamaParser::init(&args.llama_model_path).expect("failed to initialize llama parser");

        loop {
            println!("INPUT: ");
            let input = get_input()?;

            let mut tasks = Vec::new();

            if parser.parse(&input, &mut tasks).is_err() {
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

            println!("RESULTS: ");
            
            for (n, result) in results.iter().enumerate() {
                match result {
                    Ok(res) => {
                        if res.status() == Status::Success {
                            println!("{}. {}", n+1, res.response);
                        } else {
                            println!("{}. failure (agent returned: {})", n+1, res.response);
                        }
                    }
                    Err(e) => {
                        println!("{}. failure (gRPC error: {})", n+1, e);
                    }
                }
            }
        }
    }
}

fn get_input() -> Result<String, std::io::Error> {
    let stdin = std::io::stdin();

    let mut lines = stdin.lines();
    let mut input = "".to_string();
    while let Some(line) = lines.next() {
        let line = line?;
        if line.trim().is_empty() {
            break;
        }
        input.push_str("\n");
        input.push_str(&line);
    }

    Ok(input)
}
