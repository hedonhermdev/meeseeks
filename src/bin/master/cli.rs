use meeseeks::{
    common::ConnectedAgent,
    llama_parser::LlamaParser,
    master::MasterAgent,
    meeseeks_proto::{master_agent_server, Status, TaskRequest},
    tooldb::ToolDB,
};
use reqwest::Url;
use std::{
    collections::VecDeque,
    io::{self, stdin, BufRead, StdinLock, Write},
    net::SocketAddr,
    path::PathBuf,
    process::exit,
    sync::Arc,
};

use tonic::transport::Server;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct MasterCli {
    #[arg(long)]
    name: String,
    #[arg(long)]
    listen: SocketAddr,
    #[arg(long)]
    addr: String,
    #[arg(short = 'm', long = "model-path")]
    llama_model_path: PathBuf,
    #[arg(long = "tooldb-url")]
    tooldb_url: Url,
}

impl MasterCli {
    pub async fn run() -> color_eyre::Result<()> {
        let args = MasterCli::parse();

        let tooldb = ToolDB::new(args.tooldb_url)?;
        let master = Arc::new(MasterAgent::new(args.name, args.listen, tooldb));
        let master_c = master.clone();

        let _join = tokio::spawn(async move {
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

        let mut sp =
            spinners::Spinner::new(spinners::Spinners::Dots9, "Loading llama model".to_string());
        let parser =
            LlamaParser::init(&args.llama_model_path).expect("failed to initialize llama parser");
        sp.stop();
        println!("");

        let mut line = String::new();
        let mut input_tasks = VecDeque::new();
        let mut tasks = VecDeque::new();
        let mut len = 0;

        loop {
            println!("--- Command --- (enter help for a list of commands) ");
            print!("> ");
            std::io::stdout().flush()?;
            let stdin = std::io::stdin();
            len = stdin.read_line(&mut line)?;
            match line.trim() {
                "input" => {
                    println!("--- Input --- ");
                    print!("> ");
                    std::io::stdout().flush()?;

                    let stdin = std::io::stdin();
                    let len = 0;
                    loop {
                        let len = stdin.read_line(&mut line);
                        if line.trim().is_empty() {
                            break;
                        }
                        input_tasks.push_back(line.trim().to_string());
                        print!("> ");
                        std::io::stdout().flush()?;
                        line.clear();
                    }
                    infer_tasks(master.clone(), &mut input_tasks, &mut tasks, &parser).await?;

                    let mut results = Vec::new();

                    send_tasks(master.clone(), &mut tasks, &mut results).await;

                    println!("--- Results ---");
                    for (input, res) in results {
                        println!("input: {input}\nresult: {res}");
                    }
                }
                "agents" => {
                    let agents = master.list_agents();
                    println!("--- Agents ---");
                    for agent in agents {
                        println!("- {}({})", agent.name(), agent.description());
                    }
                }
                "exit" => {
                    exit(0);
                }
                "help" | _ => {
                    println!("--- Help ---");
                    println!("Available commands: ");
                    println!("\t- input: Enter a list of tasks");
                    println!("\t- agents: List connected agents");
                    println!("\t- help: Prints this message");
                    println!("\t- exit: Exits the process")
                }
            }

            line.clear();
        }
    }
}

async fn infer_tasks(
    master: Arc<MasterAgent<ToolDB>>,
    input_tasks: &mut VecDeque<String>,
    tasks: &mut VecDeque<(String, TaskRequest, Option<ConnectedAgent>)>,
    parser: &LlamaParser,
) -> color_eyre::Result<()> {
    while let Some(input) = input_tasks.pop_front() {
        match master.match_agent(&input).await {
            Ok(agent) => match parser.parse(&input, &[agent.clone()]) {
                Ok(task) => tasks.push_back((input, task, Some(agent))),
                Err(e) => tasks.push_back((
                    input,
                    TaskRequest {
                        instruction: "fail".to_string(),
                        args: vec![e.to_string()],
                    },
                    None,
                )),
            },
            Err(e) => tasks.push_back((
                input,
                TaskRequest {
                    instruction: "fail".to_string(),
                    args: vec![e.to_string()],
                },
                None,
            )),
        }
    }

    println!("--- Tasks ---");
    for (i, (input, task, agent)) in tasks.iter().enumerate() {
        if task.instruction == "fail" {
            println!("{}. input: {} task: (skipping task. failed to parse given input into a task), agent: none", i+1, input);
        } else if agent.is_none() {
            println!(
                "{}. input: {} task: (skipping task. failed to find a matching agent) agent: none",
                i + 1,
                input
            );
        } else {
            println!(
                "{}. input: {} task: {:?}, agent: {}",
                i + 1,
                input,
                task,
                agent.as_ref().unwrap().name()
            );
        }
    }

    Ok(())
}

async fn send_tasks(
    master: Arc<MasterAgent<ToolDB>>,
    tasks: &mut VecDeque<(String, TaskRequest, Option<ConnectedAgent>)>,
    results: &mut Vec<(String, String)>,
) {
    while let Some((input, task, agent)) = tasks.pop_back() {
        match task.instruction.as_str() {
            "fail" => results.push((input, "failed to complete task".to_string())),
            _ => match agent {
                Some(agent) => {
                    let res = master.send_task_to_agent(agent.name(), task).await;
                    match res {
                        Ok(result) => match result.status() {
                            Status::Success => results.push((input, result.response)),
                            Status::Failure => results.push((input, result.response)),
                        },
                        Err(_) => {
                            results.push((input, "failed to send task to agent".to_string()));
                        }
                    }
                }
                None => {
                    results.push((input, "failed to find agent to complete task".to_string()));
                }
            },
        }
    }
}
