use std::{sync::Arc, env};

use async_mutex::Mutex;
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

    pub async fn summary(&self, content: &str) -> color_eyre::Result<String> {
        let payload = serde_json::json!({
            "inputs": content,
            "parameters": { "do_sample": false }
        });
        let res: serde_json::Value = self.client.post(HF_SUMMARY_API_URL).json(&payload).send().await?.json().await?;

        let summary = match res[0].get("summary_text") {
            Some(summary) => summary,
            None => color_eyre::eyre::bail!("invalid response from huggingface api {}", res),
        };

        Ok(summary.to_string())
    }

    pub async fn qa(&self, question: &str, context: &str) -> color_eyre::Result<String> {
        let payload = serde_json::json!({
            "inputs": {
                "question": question,
                "context": context,
            }
        });
        let res: serde_json::Value = self.client.post(HF_QA_API_URL).json(&payload).send().await?.json().await?;
        
        let answer = match res.get("answer") {
            Some(answer) => answer,
            None => color_eyre::eyre::bail!("invalid response from huggingface api: {}", res),
        };
        
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
                        let summary = self.summary(&content).await.unwrap();
                        TaskResponse {
                            status: Status::Success.into(),
                            response: format!("{}", summary),
                        }
                    }
                    Err(e) => TaskResponse {
                        status: Status::Failure.into(),
                        response: format!("failed to evaluate expression: {}", e),
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
                        let answer = self.qa(&question, &content).await.unwrap();
                        TaskResponse {
                            status: Status::Success.into(),
                            response: format!("result: {}", answer),
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

#[cfg(test)]
pub mod tests {
    use std::env;

    use crate::{common::TaskExecutor, meeseeks_proto::{TaskRequest, Status}};

    use super::Wiki;

    #[tokio::test]
    async fn test_summary() -> color_eyre::Result<()> {
        env::set_var("HF_API_KEY", "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");
        let wiki = Wiki::new();
        let summary = wiki.summary("The tower is 324 metres (1,063 ft) tall, about the same height as an 81-storey building, and the tallest structure in Paris. Its base is square, measuring 125 metres (410 ft) on each side. During its construction, the Eiffel Tower surpassed the Washington Monument to become the tallest man-made structure in the world, a title it held for 41 years until the Chrysler Building in New York City was finished in 1930. It was the first structure to reach a height of 300 metres. Due to the addition of a broadcasting aerial at the top of the tower in 1957, it is now taller than the Chrysler Building by 5.2 metres (17 ft). Excluding transmitters, the Eiffel Tower is the second tallest free-standing structure in France after the Millau Viaduct.").await?;

        println!("summary: {}", summary);
        assert!(!summary.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_qa() -> color_eyre::Result<()> {
        env::set_var("HF_API_KEY", "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");
        let wiki = Wiki::new();

        let answer = wiki.qa("what is the capital of france?", "France (French: [fʁɑ̃s] Listen), officially the French Republic (French: République française [ʁepyblik fʁɑ̃sɛz]),[14] is a country located primarily in Western Europe. It also includes overseas regions and territories in the Americas and the Atlantic, Pacific and Indian Oceans,[XII] giving it one of the largest discontiguous exclusive economic zones in the world. Its metropolitan area extends from the Rhine to the Atlantic Ocean and from the Mediterranean Sea to the English Channel and the North Sea; overseas territories include French Guiana in South America, Saint Pierre and Miquelon in the North Atlantic, the French West Indies, and many islands in Oceania and the Indian Ocean. Its eighteen integral regions (five of which are overseas) span a combined area of 643,801 km2 (248,573 sq mi) and had a total population of over 68 million as of January 2023.[5][8] France is a unitary semi-presidential republic with its capital in Paris, the country's largest city and main cultural and commercial centre; other major urban areas include Marseille, Lyon, Toulouse, Lille, Bordeaux, and Nice.").await?;

        println!("answer: {}", answer);

        assert!(!answer.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_nelson_mandela() -> color_eyre::Result<()> {
        let wiki = Wiki::new();

        let res = wiki.exec(TaskRequest {
            instruction: "summary".to_string(),
            args: vec!["Nelson Mandela".to_string(), ".....".to_string()],
        }).await;

        println!("{:?}", res);

        assert_eq!(res.status, Into::<i32>::into(Status::Success));

        Ok(())
    }
}
