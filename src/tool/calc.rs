use crate::{
    common::TaskExecutor,
    meeseeks_proto::{Status, TaskRequest, TaskResponse},
};

pub struct Calculator;

#[tonic::async_trait]
impl TaskExecutor for Calculator {
    async fn exec(&self, task: TaskRequest) -> TaskResponse {
        match task.instruction.as_str() {
            "calculate" => {
                let expr = &task.args[0];
                match meval::eval_str(&expr) {
                    Ok(result) => TaskResponse {
                        status: Status::Success.into(),
                        response: format!("result: {}", result),
                    },
                    Err(e) => TaskResponse {
                        status: Status::Failure.into(),
                        response: format!("failed to evaluate expression: {}", e),
                    },
                }
            }
            _ => TaskResponse {
                status: Status::Failure.into(),
                response: "invalid instruction. available instructions are: [\"calculate\"]"
                    .to_string(),
            },
        }
    }
}
