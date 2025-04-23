use crate::config::Program;
use shlex::split;
use std::collections::HashMap;
use std::io::{Read, Result};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct Launcher {
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
        // Parse the command string into program and arguments
        let parts = split(&self.command).unwrap_or_else(|| vec![self.command.to_string()]);
        if parts.is_empty() {
            panic!("Empty command string");
        }

        // Extract the program name and arguments
        let program = &parts[0];
        let args = &parts[1..];

        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()) // Capture stderr
            .envs(self.env.iter())  // Add environment variables from the HashMap
            .spawn()
            .expect("Failed to start the program");

        println!("Started the program {:?}", child);
        
        let mut stdout = child.stdout.take().expect("Failed to get stdout");
        let mut stderr = child.stderr.take().expect("Failed to get stderr");

        let cloned_output_handler = Arc::clone(&self.output_handler);
        let cloned_status_handler = Arc::clone(&self.status_handler);
        thread::spawn(move || {
            println!("Started the program loop {:?}", child);
            loop {
                {
                    let mut buffer = String::new();
                    stdout.read_to_string(&mut buffer).expect("Failed to read stdout");
                    let mut handler = cloned_output_handler.lock().unwrap();
                    (handler)(buffer);
                }

                {
                    let mut buffer = String::new();
                    stderr.read_to_string(&mut buffer).expect("Failed to read stderr");
                    let mut handler = cloned_output_handler.lock().unwrap();
                    (handler)(buffer);
                }

                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("Program exited with status: {}", status);
                        let mut handler = cloned_status_handler.lock().unwrap();
                        (handler)(status);
                        break;
                    }
                    Ok(None) => {
                        println!("Program is still running...");
                        thread::sleep(Duration::from_secs(1)); // Wait for 1 second before checking again
                    }
                    Err(e) => {
                        eprintln!("Error occurred while waiting for the process: {}", e);
                        break;
                    }
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
    use crate::launcher::Launcher;
    use std::collections::HashMap;
    use std::option::Option;
    use std::process::ExitStatus;
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    const TIMEOUT: Duration = Duration::from_secs(5);
    
    #[test]
    fn execute_echo() {
        let status: Arc<Mutex<Option<ExitStatus>>> = Arc::new(Mutex::new(None));
        let output: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let mut launcher = Launcher::test_new("echo test".parse().unwrap(), HashMap::new());

        let output_clone = Arc::clone(&output);
        launcher.set_output_handler(move |str| {
            let mut locked = output_clone.lock().unwrap();
            if !str.is_empty() {
                *locked = Some(str.clone());
            }
        });
        
        let status_clone = Arc::clone(&status);
        launcher.set_status_handler(move |status| {
            let mut locked = status_clone.lock().unwrap();
            *locked = Some(status);
        });
        
        launcher.start().unwrap();

        let status_clone = Arc::clone(&status);
        await_condition(move || {
            let locked = status_clone.lock().unwrap();
            locked.is_some()
        });

        let locked_status = status.lock().unwrap();
        assert!(locked_status.is_some());
        assert!(locked_status.unwrap().success());
        let locked_output = output.lock().unwrap();
        assert!(locked_output.is_some());
        assert_eq!("test\n", locked_output.clone().unwrap().as_str());
    }

    #[test]
    fn execute_env() {
        let status: Arc<Mutex<Option<ExitStatus>>> = Arc::new(Mutex::new(None));
        let output: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let env = HashMap::from([("VAR1".to_string(), "VAL1".to_string())]);
        let mut launcher = Launcher::test_new("env".parse().unwrap(), env);

        let output_clone = Arc::clone(&output);
        launcher.set_output_handler(move |str| {
            let mut locked = output_clone.lock().unwrap();
            if !str.is_empty() {
                *locked = Some(str.clone());
            }
        });

        let status_clone = Arc::clone(&status);
        launcher.set_status_handler(move |status| {
            let mut locked = status_clone.lock().unwrap();
            *locked = Some(status);
        });

        launcher.start().unwrap();

        let status_clone = Arc::clone(&status);
        await_condition(move || {
            let locked = status_clone.lock().unwrap();
            locked.is_some()
        });

        let locked_output = output.lock().unwrap();
        assert!(locked_output.is_some());
        assert!(locked_output.clone().unwrap().lines().any(|line| line.contains("VAR1=VAL1")));
    }

    fn await_condition<F>(predicate: F)
    where
        F: Fn() -> bool + Send + 'static,
    {
        let start_time = Instant::now();
        let mut ok = predicate();
        while !ok && start_time.elapsed() < TIMEOUT {
            sleep(Duration::from_millis(100));
            ok = predicate();
        }
        if (!ok) {
            panic!("Timed out waiting for condition");
        }
    }
    
}