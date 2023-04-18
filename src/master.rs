use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tonic::{transport::Channel, Request, Response, Status};

use async_mutex::Mutex;

use crate::meeseeks_proto::{
    self, agent_client::AgentClient, AgentConnectRequest, AgentConnectResponse, TaskRequest,
    TaskResponse,
};

struct ConnectedAgent {
    description: String,
    addr: String,
    client: Option<AgentClient<Channel>>,
}

pub struct MasterAgent {
    name: String,
    addr: SocketAddr,
    agents: Arc<Mutex<HashMap<String, ConnectedAgent>>>,
}

impl MasterAgent {
    pub fn new(name: String, addr: SocketAddr) -> Self {
        MasterAgent {
            name,
            addr,
            agents: Mutex::new(HashMap::new()).into(),
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
}

#[tonic::async_trait]
impl meeseeks_proto::master_agent_server::MasterAgent
    for Arc<MasterAgent>
{
    async fn connect_to_master(
        &self,
        request: Request<AgentConnectRequest>,
    ) -> Result<Response<AgentConnectResponse>, Status> {
        let req = request.into_inner();
        let agent = ConnectedAgent {
            description: req.description,
            addr: req.from,
            client: None,
        };

        let agent_addr = agent.addr.clone();
        let mut agents = self.agents.lock().await;
        agents.insert(req.name, agent);
        drop(agents);

        println!("agent connected: {}", agent_addr);
        let res = AgentConnectResponse {
            status: meeseeks_proto::Status::Success.into(),
            message: "".into(),
        };

        Ok(Response::new(res))
    }
}
