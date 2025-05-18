use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::str::FromStr;
use std::{fs, io};
use toml;

/// The structure of TOML-config file.
///
#[derive(Debug, Deserialize)]
pub struct Program {
    id: String,
    command: String,
    #[serde(default)]
    superuser: bool,
    input: Option<String>,
    #[serde(default)]
    args: HashMap<String, String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    ui: UI,
}

#[derive(Default, Debug, Deserialize)]
struct UI {
    title: Option<String>,
    #[serde(default)]
    icons: Icons,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Icons {
    on: Option<String>,
    off: Option<String>,
}

impl Program {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_env(&self) -> &HashMap<String, String> {
        &self.env
    }

    pub fn get_command(&self) -> String {
        replace_args(&self.command, &self.args)
    }

    pub fn need_superuser(&self) -> bool {
        self.superuser
    }

    pub fn get_input(&self) -> Option<String> {
        Some(replace_args(self.input.as_ref()?, &self.args))
    }

    pub fn get_title(&self) -> &str {
        self.ui
            .title
            .as_deref()
            .map_or(self.id.as_str(), |title| title)
    }

    pub fn get_icon_on_path(&self) -> Option<&str> {
        self.ui.icons.on.as_deref()
    }

    pub fn get_icon_off_path(&self) -> Option<&str> {
        self.ui.icons.off.as_deref()
    }
}

fn replace_args(str: &String, args: &HashMap<String, String>) -> String {
    // Create a regex to match placeholders like $arg
    let re = Regex::new(r"\$(\w+)").expect("Failed to compile regex");

    let result = re.replace_all(&str, |caps: &regex::Captures| {
        // Extract the argument name (e.g., "arg" from "$arg")
        let arg_name = String::from_str(caps.get(1).unwrap().as_str()).unwrap();

        // Look up the argument in the map and return its value or leave it unchanged if not found
        args.get(&arg_name)
            .cloned()
            .unwrap_or_else(|| format!("${}", arg_name))
    });

    result.to_string()
}

pub fn parse_properties_file(file_path: &str) -> io::Result<Program> {
    let content = fs::read_to_string(file_path)?;
    parse_content(&content)
}

fn parse_content(content: &str) -> io::Result<Program> {
    match toml::from_str(&content) {
        Ok(program) => Ok(program),
        Err(error) => Err(io::Error::new(ErrorKind::InvalidInput, error.message())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn read_not_exists() {
        let res = parse_properties_file("invalid");
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), ErrorKind::NotFound);
    }

    #[test]
    fn read_invalid_config() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();
        // Write some data to the file
        temp_file.as_file().write_all(br#"garbage"#)?;

        let res = parse_properties_file(path);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn read_config() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();
        // Write some data to the file
        temp_file.as_file().write_all(
            br#"
          id = "id1"
          command = "command1 $arg1"
          superuser = true
          input = "arg2"
          
          [args]
          arg1 = "arg2"

          [env]
          ENVVAR = "env1"

          [ui]
          title = "title1"
          
          [ui.icons]
          on = "/some/path/to/file"
          off = "/some/path/to/file"
        "#,
        )?;

        let program = parse_properties_file(path)?;
        assert_eq!(program.get_id(), "id1");
        assert_eq!(program.get_command(), "command1 arg2");
        assert!(program.need_superuser());
        assert!(program.get_input().is_some());
        assert_eq!(program.get_input().unwrap(), "arg2");
        assert_eq!(program.get_env().get("ENVVAR").unwrap(), "env1");

        assert_eq!(program.get_title(), "title1");
        assert_eq!(program.get_icon_on_path(), Some("/some/path/to/file"));
        assert_eq!(program.get_icon_off_path(), program.get_icon_on_path());
        Ok(())
    }

    #[test]
    fn read_config_without_args_and_env() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();
        // Write some data to the file
        temp_file.as_file().write_all(
            br#"
          id = "id1"
          command = "command1"
        "#,
        )?;

        let program = parse_properties_file(path)?;
        assert_eq!(program.get_id(), "id1");
        assert_eq!(program.get_command(), "command1");
        assert!(program.get_input().is_none());
        assert!(program.get_env().is_empty());
        assert_eq!(program.get_title(), "id1");
        assert_eq!(program.get_icon_on_path(), None);
        assert_eq!(program.get_icon_off_path(), program.get_icon_on_path());
        Ok(())
    }
}
