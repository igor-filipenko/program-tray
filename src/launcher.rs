use std::collections::HashMap;
use std::io::{Read, Result};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::config::Program;

struct Launcher {
    command: String,
    env: HashMap<String, String>,
    child: Option<Child>,
    output_handler: Arc<Mutex<dyn FnMut(String) + Send>>,
    status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>,
}

impl Launcher {
    
    pub fn new(program: &Program) -> Self {
        Launcher {
            command: program.command().clone(),
            env: program.env().clone(),
            child: None,
            output_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
            status_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
        }
    }

    #[cfg(test)]
    pub fn test_new(command: String, env: HashMap<String, String>) -> Self {
        Launcher {
            command: command.clone(),
            env: env.clone(),
            child: None,
            output_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
            status_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
        }
    }
    
    pub fn set_output_handler<F>(&mut self, handler: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        self.output_handler = Arc::new(Mutex::new(handler));
    }

    pub fn set_status_handler<F>(&mut self, handler: F)
    where
        F: FnMut(ExitStatus) + Send + 'static,
    {
        self.status_handler = Arc::new(Mutex::new(handler));
    }

    pub fn start(&mut self) -> Result<()> {
        let mut child = Command::new(self.command.clone())
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()) // Capture stderr
            .envs(self.env.iter())  // Add environment variables from the HashMap
            .spawn()
            .expect("Failed to start the program");

        let mut stdout = child.stdout.take().expect("Failed to get stdout");
        let mut stderr = child.stderr.take().expect("Failed to get stderr");

        let handler = Arc::clone(&self.output_handler);
        thread::spawn(move || {
            let mut buffer = String::new();
            stdout.read_to_string(&mut buffer).expect("Failed to read stdout");
            let mut handler = handler.lock().unwrap();
            (handler)(buffer);
        });
        
        let handler = Arc::clone(&self.output_handler);
        thread::spawn(move || {
            let mut buffer = String::new();
            stderr.read_to_string(&mut buffer).expect("Failed to read stderr");
            let mut handler = handler.lock().unwrap();
            (handler)(buffer);
        });

        let handler = Arc::clone(&self.status_handler);
        thread::spawn(move || {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("Program exited with status: {}", status);
                    let mut handler = handler.lock().unwrap();
                    (handler)(status);
                }
                Ok(None) => {
                    println!("Program is still running...");
                    thread::sleep(Duration::from_secs(1)); // Wait for 1 second before checking again
                }
                Err(e) => {
                    eprintln!("Error occurred while waiting for the process: {}", e);
                }
            }
        });

        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            child.kill()?;
            self.child = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::thread::sleep;
    use std::time::Duration;
    use crate::launcher::Launcher;

    #[test]
    fn execute_echo() {
        let mut launcher = Launcher::test_new("echo".parse().unwrap(), HashMap::new());
        launcher.set_output_handler(|str| {
            println!("output: {}", str);
        });
        launcher.set_status_handler(|status| {
            println!("status: {}", status);
        });
        launcher.start().unwrap();
        sleep(Duration::from_secs(1));
    }
    
}