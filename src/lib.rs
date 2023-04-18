pub mod agent;
pub mod common;
pub mod error;
pub mod llama_parser;
pub mod master;
pub mod tool;

pub mod meeseeks_proto {
    tonic::include_proto!("meeseeks_v1");
}
