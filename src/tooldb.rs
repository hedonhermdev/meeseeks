use color_eyre::eyre::bail;
use reqwest::{Url, Client};

use crate::common::{AgentMatcher, ConnectedAgent};

pub struct ToolDB {
    client: Client,
    addr: Url,
}

impl ToolDB {
    pub fn new(addr: Url) -> color_eyre::Result<ToolDB> {
        let client = reqwest::Client::new();

        Ok(Self { client, addr })
    }
}

#[tonic::async_trait]
impl AgentMatcher for ToolDB {
    async fn add_agent(&self, agent: ConnectedAgent) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::json!({
            "tool": {
                "name": agent.name,
                "commands": agent.commands,
                "examples": agent.examples,
            }
        });

        let addr = self.addr.join("/tool/add").unwrap();
        let res: serde_json::Value = self.client.post(addr).json(&payload).send().await?.json().await?;

        Ok(())
    }

    async fn match_agent(&self, task: &str) -> Result<String, Box<dyn std::error::Error>> {
        let addr = self.addr.join("/tool/match").unwrap();
        let res: serde_json::Value = self.client.get(addr).query(&[("task", task)]).send().await?.json().await?;


        match res.get("name") {
            Some(name) => Ok(name.as_str().unwrap().to_string()),
            None => {
                Ok("".to_string())
            }
        }
    }
}
