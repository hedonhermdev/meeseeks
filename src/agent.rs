use tonic::{transport::Channel, Request, Response, Status};

use crate::{
    common::TaskExecutor,
    meeseeks_proto::{
        self, master_agent_client::MasterAgentClient, AgentConnectRequest, TaskRequest,
        TaskResponse,
    },
};

use crate::error::Result;

pub struct Agent<Executor: TaskExecutor> {
    name: String,
    description: String,
    addr: String,
    master_addr: Option<String>,
    client: Option<MasterAgentClient<Channel>>,
    executor: Executor,
    commands: Vec<String>,
    examples: String,
}

impl<T: TaskExecutor> Agent<T> {
    pub fn new(name: String, description: String, addr: String, executor: T, commands: Vec<String>, examples: String) -> Self {
        Agent {
            name,
            description,
            addr,
            executor,
            master_addr: None,
            client: None,
            commands,
            examples,
        }
    }

    pub async fn connect_to_master(&mut self, master_addr: String) -> Result<()> {
        self.master_addr = Some(master_addr.clone());

        tracing::debug!("trying to connect to master: {}", master_addr);

        let mut client = MasterAgentClient::connect(master_addr.clone()).await?;
        let res = client
            .connect_to_master(AgentConnectRequest {
                name: self.name.clone(),
                description: self.description.clone(),
                from: self.addr.to_string(),
                examples: self.examples.clone(),
                commands: self.commands.clone(),
            })
            .await?;

        tracing::info!(
            "master returned response status: {}",
            res.into_inner().status
        );
        self.client = Some(client);
        tracing::info!("connected to master: {}", master_addr);

        Ok(())
    }
}

#[tonic::async_trait]
impl<Executor: TaskExecutor + Send + Sync + 'static> meeseeks_proto::agent_server::Agent
    for Agent<Executor>
{
    async fn exec_task(
        &self,
        request: Request<TaskRequest>,
    ) -> std::result::Result<Response<TaskResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!("executing task: {:?}", req);
        let res = self.executor.exec(req).await;

        Ok(Response::new(res))
    }
}
