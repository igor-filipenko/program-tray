use std::collections::HashMap;
use toml;
use std::fs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Program {
    title: String,
    command: String,
    args: HashMap<String, String>,
    env: HashMap<String, String>,
}

pub fn parse_properties_file(file_path: &str) -> Result<Program, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path);
    match content {
        Ok(content) => {
            println!("Read file {}", file_path);
            parse_content(&content)
        },
        Err(error) => {
            println!("Error reading file: {}", error);
            Err(Box::new(error))
        },
    }
}

fn parse_content(content: &str) -> Result<Program, Box<dyn std::error::Error>> {
    match toml::from_str(&content) {
        Ok(program) => Ok(program),
        Err(error) => {
            println!("Error parsing TOML file: {}", error);
            Err(Box::new(error))
        }
    }
}
