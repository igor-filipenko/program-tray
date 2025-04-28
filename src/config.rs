use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use toml;
use regex::Regex;

/// The structure of TOML-config file.
/// 
#[derive(Debug, Deserialize)]
pub struct Program {
    title: String,
    command: String,
    input: Option<String>,
    args: HashMap<String, String>,
    env: HashMap<String, String>,
}

impl Program {

    pub fn title(&self) -> &str { &self.title }
    pub fn args(&self) -> &HashMap<String, String> { &self.args }
    pub fn env(&self) -> &HashMap<String, String> { &self.env }

    pub fn command(&self) -> String {
        // Create a regex to match placeholders like $arg
        let re = Regex::new(r"\$(\w+)").unwrap();

        // Replace each match with the corresponding value from the map
        re.replace_all(&self.command, |caps: &regex::Captures| {
            // Extract the argument name (e.g., "arg" from "$arg")
            let arg_name = String::from_str(caps.get(1).unwrap().as_str()).unwrap();

            // Look up the argument in the map and return its value or leave it unchanged if not found
            self.args.get(&arg_name).cloned().unwrap_or_else(|| format!("${}", arg_name))
        }).to_string()
    }

    pub fn input(&self) -> Option<String> {
        if self.input.is_none() {
            return None;
        }
        
        // Create a regex to match placeholders like $arg
        let re = Regex::new(r"\$(\w+)").unwrap();

        // Replace each match with the corresponding value from the map
        let input = self.input.clone().unwrap();
        let result = re.replace_all(&input, |caps: &regex::Captures| {
            // Extract the argument name (e.g., "arg" from "$arg")
            let arg_name = String::from_str(caps.get(1).unwrap().as_str()).unwrap();

            // Look up the argument in the map and return its value or leave it unchanged if not found
            self.args.get(&arg_name).cloned().unwrap_or_else(|| format!("${}", arg_name))
        }).to_string();
        Some(result)
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
    match toml::from_str(&content) {
        Ok(program) => Ok(program),
        Err(error) => {
            println!("Error parsing TOML file: {}", error);
            Err(Box::new(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn read_config() -> io::Result<()> {
        // Create a temporary file
        let temp_file = NamedTempFile::new()?;

        // Write some data to the file
        temp_file.as_file().write_all(br#"
          title = "title1"
          command = "command1 $arg1"
          input = "arg2"
          
          [args]
          arg1 = "arg2"

          [env]
          ENVVAR = "env1"
        "#)?;

        let program = match parse_properties_file(temp_file.path().to_str().unwrap()) {
            Ok(program) => program,
            Err(_) => panic!("Error reading file"),
        };
        assert_eq!(program.title(), "title1");
        assert_eq!(program.command(), "command1 arg2");
        assert!(program.input().is_some());
        assert_eq!(program.input().unwrap(), "arg2");
        assert_eq!(program.args().get("arg1").unwrap(), "arg2");
        assert_eq!(program.env().get("ENVVAR").unwrap(), "env1");
        Ok(())
    }

}