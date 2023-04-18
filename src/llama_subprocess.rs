use std::path::Path;

use subprocess::{Popen, PopenConfig, Redirection};

struct LlamaSubprocess {
    process: Popen,
}

impl LlamaSubprocess {
    pub fn spawn(model_path: &Path, prompt_template: &Path) -> color_eyre::Result<Self> {
        let argv = [
            "./llama.cpp/main",
            "-m",
            model_path.to_str().unwrap(),
            "-f",
            prompt_template.to_str().unwrap(),
            "-i",
            "--interactive-first",
            "--top_k",
            "10000",
            "--temp",
            "0.2",
            "--repeat_penalty",
            "1",
            "-t",
            "7",
            "-c",
            "2048",
            "-r",
            "\"### TASKS ###\"",
            "--in-prefix",
            " ",
            "-n",
            "-1",
        ];
        let config = PopenConfig {
            stdin: Redirection::Pipe,
            stdout: Redirection::Pipe,
            stderr: Redirection::None,
            ..Default::default()
        };

        let process = Popen::create(&argv, config)?;

        Ok(Self { process })
    }

    pub fn send_prompt(prompt: String) {

    }
}
