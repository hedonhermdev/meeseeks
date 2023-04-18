use crate::meeseeks_proto::{TaskRequest, TaskResponse};

pub struct Task {
    pub instruction: String,
    pub args: Vec<String>,
}

#[tonic::async_trait]
pub trait TaskParser {
    async fn parse(&self, input: &str, tasks: &mut Vec<TaskRequest>);
}

#[tonic::async_trait]
pub trait TaskExecutor {
    async fn exec(&self, req: TaskRequest) -> TaskResponse;
}
