use color_eyre::eyre::bail;
use dyn_fmt::AsStrFormatExt;
use rand::SeedableRng;
use std::{cell::RefCell, collections::VecDeque, path::Path};

use llama_rs::{InferenceParameters, InferenceSessionParameters};

use crate::{common::ConnectedAgent, meeseeks_proto::TaskRequest};

lazy_static::lazy_static! {
static ref RE: regex::Regex = regex::Regex::new(r"Action: (?P<command>\w+)\[(?P<args>.*?)\]").unwrap();
}

const PROMPT_TEMPLATE: &'static str = r#"
You run in a loop of Input, Thought and Action. I will provide the Input and you are supposed to use only Thought or Action. Use Thought to describe your thoughts about the question you have been asked. If there is no tool available, you can just respond with NONE. 
Use Action to run one of these actions available to you:
{}

{}
Input: {}
"#;

pub struct LlamaParser {
    model: llama_rs::Model,
    vocab: llama_rs::Vocabulary,
    inference_params: InferenceParameters,
}

impl LlamaParser {
    pub fn init(model_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (model, vocab) = llama_rs::Model::load(model_path, 512, |_| ())?;
        let inference_params = InferenceParameters {
            temperature: 0.1,
            top_k: 10000,
            n_threads: num_cpus::get(),
            ..Default::default()
        };

        Ok(Self {
            model,
            vocab,
            inference_params,
        })
    }

    pub fn parse(
        &self,
        input: &str,
        agents: &[ConnectedAgent],
    ) -> color_eyre::Result<TaskRequest> {
        let mut params = InferenceSessionParameters::default();
        params.repetition_penalty_last_n = 1;
        let mut session = self.model.start_session(params);

        let mut rng = rand::rngs::StdRng::from_entropy();

        let prompt = construct_prompt(PROMPT_TEMPLATE, agents, input);

        let text = RefCell::new(String::new());

        let mut sp = spinners::Spinner::new(
            spinners::Spinners::Dots9,
            "Running inference on input".into(),
        );
        
        session.inference_with_prompt(
            &self.model,
            &self.vocab,
            &self.inference_params,
            &prompt,
            Some(512),
            &mut rng,
            |new_text| {
                text.borrow_mut().push_str(new_text);
                if text.borrow().len() > prompt.len() {
                    tracing::debug!("llama is generating output: {}", new_text);
                    if text.borrow()[prompt.len()..].contains("Input") {
                        return Err(LlamaInferenceError::Done);
                    }
                }
                Ok(())
            },
        );
        sp.stop();
        println!("");

        let text = text.into_inner();
        let text = text[prompt.len()..].trim();
        let text = text.split('\n').nth(1).unwrap_or_else(|| {
            ""
        });

        tracing::info!("llama output: {:?}", text);

        match RE.captures(text) {
            Some(caps) => {
                let caps = dbg!(caps);
                let command = caps.get(1);
                let args = caps.get(2);

                match (command, args) {
                    (Some(instruction), Some(args)) => {
                        let instruction = instruction.as_str().to_owned();
                        let args = vec![args.as_str().to_owned(), input.to_owned()];

                        let task = TaskRequest { instruction, args };
                        tracing::debug!("inferred new task: {:?}", task);

                        Ok(task)
                    }
                    _ => {
                        bail!("failed to infer task")
                    }
                }
            }
            None => {
                bail!("failed to infer task")
            }
        }
    }
}

fn construct_prompt(template: &str, agents: &[ConnectedAgent], input: &str) -> String {
    let mut list_tools = String::new();
    let mut list_examples = String::new();

    for agent in agents {
        for command in &agent.commands {
            list_tools.push_str(&format!("- {}\n", command));
        }
        list_examples.push('\n');
        list_examples.push_str(&agent.examples.trim());
    }

    template.format([&list_tools, &list_examples, input.trim()])
}

#[derive(Debug, thiserror::Error)]
enum LlamaInferenceError {
    #[error("done")]
    Done,
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use super::LlamaParser;

    pub fn test_parser() -> color_eyre::Result<()> {
        let path = PathBuf::from_str("./models/burns-lora-q4.bin").unwrap();
        let parser = LlamaParser::init(&path).unwrap();
        Ok(())
    }
}
