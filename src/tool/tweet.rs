use dyn_fmt::AsStrFormatExt;

use crate::{
    common::TaskExecutor,
    meeseeks_proto::{TaskRequest, TaskResponse, Status},
};

const PROMPT_TEMPLATE: &'static str = r#"
    You are aTranscript of a dialog, where the User interacts with an Assistant named Tweetu. Tweetu is helpful, kind, honest and a creative writer. Tweetu specialises in writing tweets for the user. Tweets are short creative pieces of text that have a limit of 140 characters and can contain hashtags and mention other users.

User: Can you write a tweet about the benefits of drinking coffee?
Tweetu: Starting your day with a cup of coffee not only provides an energy boost, but also offers a range of health benefits, such as improving focus, reducing the risk of diseases, and even enhancing athletic performance. #coffeebenefits #healthylifestyle
User: Please write a tweet about GitHub.
Tweetu: Harness the power of #GitHub!. Collaborative coding made easy. Version control for seamless progress. Open-source treasure trove. Integrated issue tracking & project management. Community-driven knowledge & support. Unleash your dev potential today! #GitGoing #DevLife
User: Write a tweet about {}.  
Tweetu: 
"#;
const OPENAI_COMPLETION_API_URL: &'static str = "https://api.openai.com/v1/completions";
const OPENAI_MODEL_NAME: &'static str = "text-davinci-003";

const COMMANDS: &[&'static str] = &["tweet(topic)"];
const EXAMPLES: &'static str = include_str!("../../prompts/tweetu.txt");

pub struct Tweetu {
    client: reqwest::Client,
}


impl Tweetu {
    pub fn new() -> Self {
        let auth = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set. Required to initialize tweetu which uses the OpenAI API");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth)).unwrap());
        headers.insert("Content-Type", reqwest::header::HeaderValue::from_str("application/json").unwrap());

        let client = reqwest::ClientBuilder::new().default_headers(headers).build().unwrap();
        Self {
            client,
        }
    }
    
    async fn get_tweet(&self, topic: &str) -> color_eyre::Result<String> {
        let prompt = PROMPT_TEMPLATE.format(&[topic]);

        let payload = serde_json::json!({
            "model": OPENAI_MODEL_NAME,
            "prompt": prompt,
            "max_tokens": 248,
            "temperature": 0.7,
            "top_p": 1,
            "n": 1,
            "stream": false,
        });

        let res: serde_json::Value = self.client.post(OPENAI_COMPLETION_API_URL).json(&payload).send().await?.json().await?;

        println!("{:?}", res);
        let tweet = match res.get("choices") {
            Some(choices) => {
                choices[0].get("text").unwrap()
            }
            None => {
                color_eyre::eyre::bail!("OpenAI API returned an invalid response")
            }
        };

        Ok(tweet.to_string())
    }
}

#[tonic::async_trait]
impl TaskExecutor for Tweetu {
    async fn exec(&self, task: TaskRequest) -> TaskResponse {
        match task.instruction.as_str() {
            "twee" | "tweet" | "tweeit" | "tweeet" => {
                let topic = &task.args[0];
                match self.get_tweet(topic).await {
                    Ok(tweet) => TaskResponse {
                        status: Status::Success.into(),
                        response: tweet
                    },
                    Err(e) => TaskResponse{
                        status: Status::Failure.into(),
                        response: format!("failed to generate tweet: {}", e)
                    }
                }
            }
            _ => TaskResponse {
                status: Status::Failure.into(),
                response: "invalid instruction. available instructions are: [\"tweet\"]".to_string()
            }
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
mod tests {
    use crate::{tool::Tweetu, common::TaskExecutor, meeseeks_proto::{TaskRequest, Status}};

    #[tokio::test]
    pub async fn test_tweetu() -> color_eyre::Result<()> {
        std::env::set_var("OPENAI_API_KEY", "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");

        let tweetu = Tweetu::new();

        let res = tweetu.exec(TaskRequest {
            instruction: "tweet".to_string(),
            args: vec!["Elon Musk".to_string(), "write a tweet about Elon Musk".to_string()],
        }).await;

        println!("{}", res.response);
        assert_eq!(res.status, Into::<i32>::into(Status::Success));

        Ok(())
    }
}
