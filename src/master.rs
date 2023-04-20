use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use color_eyre::eyre::bail;
use tonic::{Request, Response, Status};

use async_mutex::Mutex;

use crate::{
    common::{ConnectedAgent, AgentMatcher},
    meeseeks_proto::{
        self, agent_client::AgentClient, AgentConnectRequest, AgentConnectResponse, AgentInfo,
        ConnectedAgentInfo, EmptyParams, TaskRequest, TaskResponse,
    },
};

pub struct MasterAgent<Matcher: AgentMatcher> {
    name: String,
    addr: SocketAddr,
    agents: Arc<Mutex<HashMap<String, ConnectedAgent>>>,
    matcher: Matcher,
}

impl<Matcher: AgentMatcher + Send + Sync> MasterAgent<Matcher> {
    pub fn new(name: String, addr: SocketAddr, matcher: Matcher) -> Self {
        MasterAgent {
            name,
            addr,
            matcher,
            agents: Mutex::new(HashMap::new()).into(),
        }
    }
    
    pub async fn match_agent(&self, input_task: &str) -> color_eyre::Result<ConnectedAgent> {
        match self.matcher.match_agent(input_task).await {
            Ok(agent_name) => {
                let connected_agents = self.agents.lock().await;
                match connected_agents.get(&agent_name) {
                    Some(agent) => Ok(agent.clone()),
                    None => {
                        bail!("agent with name {} not found", agent_name);
                    }
                }
            }
            Err(e) => {
                bail!("failed to match input task to agent: {}", e);
            }
        }
        

    }

    pub async fn send_task_to_agent<'a>(
        &'a self,
        name: &str,
        task: TaskRequest,
    ) -> Result<TaskResponse, Box<dyn std::error::Error>> {
        let mut agents = self.agents.lock().await;

        let agent = agents
            .get_mut(name)
            .ok_or("failed to get client with name: {name}")?;

        tracing::info!("sending task to agent: {}", name);
        tracing::debug!("task: {:?}, agent: {}", task, name);
        if agent.client.is_none() {
            tracing::debug!("agent \"{}\" client is not connected. connecting...", name);
            let client = AgentClient::connect(agent.addr.clone()).await?;
            agent.client = Some(client);
        }
        tracing::debug!("agent \"{}\" client connected.", name);

        let mut client = agent.client.take().unwrap();

        let result = client.exec_task(task).await?;

        agent.client = Some(client);

        Ok(result.into_inner())
    }

    pub fn list_agents(&self) -> Vec<ConnectedAgent> {
        let agents = futures::executor::block_on(self.agents.lock());

        agents.values().map(|x| x.clone()).collect()
    }
}

#[tonic::async_trait]
impl<Matcher: AgentMatcher + Send + Sync + 'static> meeseeks_proto::master_agent_server::MasterAgent for Arc<MasterAgent<Matcher>> {
    async fn connect_to_master(
        &self,
        request: Request<AgentConnectRequest>,
    ) -> Result<Response<AgentConnectResponse>, Status> {
        let req = request.into_inner();
        let agent = ConnectedAgent {
            name: req.name.clone(),
            description: req.description,
            addr: req.from,
            examples: req.examples,
            commands: req.commands,
            client: None,
        };

        let agent_addr = agent.addr.clone();
        let mut agents = self.agents.lock().await;
        self.matcher.add_agent(agent.clone()).await.map_err(|_| tonic::Status::new(tonic::Code::Unavailable, "failed to add agent to tooldb"))?;
        agents.insert(req.name, agent);
        drop(agents);

        let res = AgentConnectResponse {
            status: meeseeks_proto::Status::Success.into(),
            message: "".into(),
        };

        Ok(Response::new(res))
    }

    async fn connected_agents(
        &self,
        _: Request<EmptyParams>,
    ) -> Result<Response<ConnectedAgentInfo>, Status> {
        let agents = self.agents.lock().await;

        let mut connected_agents = Vec::new();
        for (name, agent) in agents.iter() {
            let info = AgentInfo {
                name: name.to_string(),
                description: agent.description.to_string(),
            };
            connected_agents.push(info);
        }

        Ok(Response::new(ConnectedAgentInfo {
            agents: connected_agents,
        }))
    }
}
