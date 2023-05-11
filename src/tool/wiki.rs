use std::{env};


use wikipedia::Wikipedia;

use crate::{
    common::TaskExecutor,
    meeseeks_proto::{Status, TaskRequest, TaskResponse},
};

const HF_QA_API_URL: &'static str = "https://api-inference.huggingface.co/models/deepset/roberta-base-squad2";
const HF_SUMMARY_API_URL: &'static str = "https://api-inference.huggingface.co/models/facebook/bart-large-cnn";

const COMMANDS: &[&'static str] = &["summary(topic)", "question(query)"];
const EXAMPLES: &'static str = include_str!("../../prompts/wiki.txt");


pub struct Wiki {
    wiki: Wikipedia<wikipedia::http::default::Client>,
    client: reqwest::Client,
}

impl Wiki {
    pub fn new() -> Self {
        let wiki = Wikipedia::default();
        let mut headers = reqwest::header::HeaderMap::new();
        let auth = env::var("HF_API_KEY").expect("HF_API_KEY is not set. Required to use HF APIs for wiki client");
        headers.insert("Authorization", reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth)).unwrap());
        let client = reqwest::ClientBuilder::new().default_headers(headers).build().unwrap();

        Self {
            wiki,
            client,
        }
    }

    pub async fn summary(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let payload = serde_json::json!({
            "inputs": content,
            "parameters": { "do_sample": false }
        });
        let res: serde_json::Value = self.client.post(HF_SUMMARY_API_URL).json(&payload).send().await?.json().await?;

        let summary = res[0].get("summary_text").ok_or(WikiError::HFApiError(res.clone()))?;

        Ok(summary.to_string())
    }

    pub async fn qa(&self, question: &str, context: &str) -> Result<String, Box<dyn std::error::Error>> {
        let payload = serde_json::json!({
            "inputs": {
                "question": question,
                "context": context,
            }
        });
        let res: serde_json::Value = self.client.post(HF_QA_API_URL).json(&payload).send().await?.json().await?;
        
        let answer = res[0].get("answer").ok_or(WikiError::HFApiError(res.clone()))?;
        
        Ok(answer.to_string())
    }
}

#[tonic::async_trait]
impl TaskExecutor for Wiki {
    async fn exec(&self, task: TaskRequest) -> TaskResponse {
        if task.args.len() < 2 {
            return TaskResponse {
                status: Status::Failure.into(),
                response: "Invalid instruction provided. Missing args".to_string(),
            };
        }
        match task.instruction.as_str() {
            "summary" => {
                let query = &task.args[0];
                let pages = match self.wiki.search(query) {
                    Ok(pages) if pages.len() > 0 => pages,
                    _ => {
                        return TaskResponse {
                            status: Status::Failure.into(),
                            response: format!("failed to search wikipedia")  
                        };
                    }
                };

                let page = self.wiki.page_from_title(pages[0].to_string());
                match page.get_summary() {
                    Ok(content) => {
                        match self.summary(&content).await {
                            Ok(summary) => {
                                TaskResponse {
                                    status: Status::Success.into(),
                                    response: format!("{}", summary),
                                }
                            }
                            Err(e) => TaskResponse {
                                status: Status::Failure.into(),
                                response: format!("failed to get summary: {}", e),
                            },
                        }
                    }
                    Err(e) => TaskResponse {
                        status: Status::Failure.into(),
                        response: format!("failed to fetch wiki page: {}", e),
                    },
                }
            }
            "question" | "search" => {
                let query = &task.args[0];
                let question = &task.args[1];
                let pages = match self.wiki.search(query) {
                    Ok(pages) if pages.len() > 0 => pages,
                    _ => {
                        return TaskResponse {
                            status: Status::Failure.into(),
                            response: format!("failed to search wikipedia")  
                        };
                    }
                };

                let page = self.wiki.page_from_title(pages[0].to_string());
                match page.get_summary() {
                    Ok(content) => {
                        match self.qa(&question, &content).await {
                            Ok(answer) => {
                                TaskResponse {
                                    status: Status::Success.into(),
                                    response: format!("result: {}", answer),
                                }
                            },
                            Err(e) => {
                                TaskResponse {
                                    status: Status::Failure.into(),
                                    response: format!("failed to answer question: {}", e),
                                }
                            }
                        }
                    }
                    Err(e) => TaskResponse {
                        status: Status::Failure.into(),
                        response: format!("failed to answer question: {}", e),
                    },
                }
            }
            _ => TaskResponse {
                status: Status::Failure.into(),
                response:
                    "invalid instruction. available instructions are: [\"summary\", \"question\", \"search\"]"
                        .to_string(),
            },
        }
    }

    fn commands<'a>(&self) -> &'a[&'a str] {
        COMMANDS
    }

    fn examples<'a>(&self) -> &'a str {
        EXAMPLES
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WikiError {
    #[error("invalid response from HF API")]
    HFApiError(serde_json::Value)
}
