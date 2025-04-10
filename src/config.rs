use std::collections::HashMap;
use properties_file_parser::{parse_properties, Property};
use std::fs;

pub struct Program {
    title: String,
    command: String,
    env: HashMap<String, String>,
}

impl Program {
    // Method to convert the Program struct into a String representation
    pub fn to_string(&self) -> String {
        let mut result = format!("Title: {}\nCommand: {}\nEnvironment Variables:\n", self.title, self.command);

        for (key, value) in &self.env {
            result.push_str(&format!("  {}: {}\n", key, value));
        }

        result
    }
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
    match parse_properties(content) {
        Ok(props) => {
            println!("Parsed properties");
            convert(props)
        },
        Err(error) => {
            eprintln!("Can't parse properties: {}", content);
            Err(Box::new(error))
        }
    }
}

fn convert(properties: Vec<Property>) -> Result<Program, Box<dyn std::error::Error>> {
    let mut title = None;
    let mut command = None;
    let mut env = HashMap::new();

    // Iterate over the properties and populate the fields
    for prop in properties {
        match prop.key.as_str() {
            "title" => title = Some(prop.value),
            "command" => command = Some(prop.value),
            key if key.starts_with("env.") => {
                let env_key = key.trim_start_matches("env.");
                env.insert(env_key.to_string(), prop.value);
            }
            _ => {
                // Ignore unknown keys or log a warning
                println!("Warning: Unknown property key '{}'", prop.key);
            }
        }
    }

    // Ensure required fields are present
    let title = title.ok_or("Missing 'title' in properties")?;
    let command = command.ok_or("Missing 'command' in properties")?;

    // Construct the Program struct
    Ok(Program { title, command, env })
}