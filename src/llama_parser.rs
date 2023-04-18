use async_mutex::Mutex;
use dyn_fmt::AsStrFormatExt;
use rand::SeedableRng;
use std::{cell::RefCell, convert::Infallible, path::Path, sync::Arc, borrow::Borrow};

use llama_rs::{InferenceParameters, InferenceSessionParameters};

use crate::{common::TaskParser, meeseeks_proto::TaskRequest};

lazy_static::lazy_static! {
    static ref RE: regex::Regex = regex::Regex::new(r"(?P<command>\w+)\((?P<args>.*?)\)").unwrap();
}

const PROMPT_TEMPLATE: &'static str = r#"
You are given a list of tools followed by a list of tasks. Select the most appropriate tool to complete each task. You can only use the given tools and nothing else. If no tool can complete the given task, respond with none.
### Tools ###
- calculate(expression)
- tweet(topic)
- search(query)
- summary(topic)
### Tasks ###
1. what is 999 - 1?
2. write a tweet about Github's new feature release.
3. write a short paragraph about Elon Musk.
4. who was the first prime minister of UK?
### Response ###
1. calculate(999 - 1)
2. tweet(Github's new feature release)
3. summary(Elon Musk)
4. search(first prime minister of UK)
### Tasks ###
{}
### Response ###
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

    pub fn parse(&self, input: &str, output_tasks: &mut Vec<TaskRequest>) -> color_eyre::Result<()> {
        let mut params = InferenceSessionParameters::default();
        params.repetition_penalty_last_n = 1;
        let mut session = self.model.start_session(params);

        let mut rng = rand::rngs::StdRng::from_entropy();

        let prompt = PROMPT_TEMPLATE.format(&[input]);

        let text = RefCell::new(String::new());

        tracing::debug!("Trying to parse input:\n{}", input);
        tracing::debug!("Using prompt: \n{}", prompt);

        let mut sp = spinners::Spinner::new(spinners::Spinners::Dots9, "Running inference on input".into());
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
                    if text.borrow()[prompt.len()..].contains("Tasks") {
                        return Err(LlamaInferenceError::Done);
                    }
                }
                Ok(())
            },
        );
        sp.stop();

        let text = text.into_inner();
        let text = text[prompt.len()..].trim();

        tracing::debug!("llama output: {:?}", text);

        let input_tasks = input.trim().split('\n');
        let tasks = text.split('\n');

        tasks.zip(input_tasks).for_each(|(line, input_line)| {
            match RE.captures(line) {
                Some(caps) => {
                    let command = caps.get(1);
                    let args = caps.get(2);

                    match (command, args) {
                        (Some(instruction), Some(args)) => {
                            let instruction = instruction.as_str().to_owned();
                            let args = vec![args.as_str().to_owned(), input_line.to_owned()];

                            let task = TaskRequest { instruction, args };
                            tracing::debug!("inferred new task: {:?}", task);
                            output_tasks.push(task);
                        }
                        _ => {
                            tracing::debug!("skipping task: {} since it does not match with any available tools", line);
                        }
                    }
                }
                None => {
                    tracing::debug!("skipping task: {} since it does not match with any available tools", line);
                }
            }

        });

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum LlamaInferenceError {
    #[error("done")]
    Done
}
