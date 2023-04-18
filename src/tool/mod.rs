mod calc;
mod tweet;
mod wiki;

pub use calc::Calculator;
pub use tweet::Tweetu;
pub use wiki::Wiki;

use crate::{
    common::TaskExecutor,
    meeseeks_proto::{TaskRequest, TaskResponse},
};

pub enum Tool {
    Calculator(Calculator),
    Tweetu(Tweetu),
    Wiki(Wiki),
}

#[tonic::async_trait]
impl TaskExecutor for Tool {
    async fn exec(&self, req: TaskRequest) -> TaskResponse {
        match self {
            Tool::Calculator(calc) => calc.exec(req).await,
            Tool::Tweetu(tweetu) => tweetu.exec(req).await,
            Tool::Wiki(wiki) => wiki.exec(req).await,
        }
    }
}
