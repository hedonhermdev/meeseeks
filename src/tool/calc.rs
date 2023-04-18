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

#[cfg(test)]
mod tests {
    use crate::{tool::Calculator, common::TaskExecutor, meeseeks_proto::{TaskRequest, Status}};

    #[tokio::test]
    pub async fn test_calculator() {
        let calc = Calculator;
        let res = calc.exec(TaskRequest {
            instruction: "calculate".to_string(),
            args: vec!["17 * 9".to_string(), "what is 17 * 9?".to_string()],
        }).await;

        println!("{:?}", res);

        assert_eq!(res.status, Into::<i32>::into(Status::Success))
    }
}
