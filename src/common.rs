use tonic::transport::Channel;

use crate::meeseeks_proto::{TaskRequest, TaskResponse, agent_client::AgentClient};


#[derive(Clone)]
pub struct ConnectedAgent {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) addr: String,
    pub(crate) client: Option<AgentClient<Channel>>,
    pub(crate) examples: String,
    pub(crate) commands: Vec<String>,
}

impl ConnectedAgent {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }
}


// pub struct Task {
//     pub instruction: String,
//     pub args: Vec<String>,
// }

#[tonic::async_trait]
pub trait TaskParser {
    async fn parse(&self, input: &str, tasks: &mut Vec<TaskRequest>);
}

#[tonic::async_trait]
pub trait TaskExecutor {
    async fn exec(&self, req: TaskRequest) -> TaskResponse;

    fn commands<'a>(&self) -> &'a[&'a str];

    fn examples<'a>(&self) -> &'a str;
}

#[tonic::async_trait]
pub trait AgentMatcher {

    async fn match_agent(&self, task: &str) -> Result<String, Box<dyn std::error::Error>>;

    async fn add_agent(&self, agent: ConnectedAgent) -> Result<(), Box<dyn std::error::Error>>;
}
