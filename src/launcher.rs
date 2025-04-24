use crate::config::Program;
use shlex::split;
use std::collections::HashMap;
use std::io::{BufReader, Read, Result, Write};
use std::ops::{Deref, DerefMut};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::time::Duration;

pub struct Launcher {
    command: String,
    input: Option<String>,
    env: HashMap<String, String>,
    child: Option<Child>,
    running_flag: Arc<Mutex<bool>>,
    output_handler: Arc<Mutex<dyn FnMut(String) + Send>>,
    status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>,
}

impl Launcher {
    
    pub fn new(program: &Program) -> Self {
        Launcher {
            command: program.command().clone(),
            input: program.input().clone(),
            env: program.env().clone(),
            child: None,
            running_flag: Arc::new(Mutex::new(false)),
            output_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
            status_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
        }
    }

    #[cfg(test)]
    pub fn test_new(command: String, env: HashMap<String, String>) -> Self {
        Launcher {
            command: command.clone(),
            input: None,
            env: env.clone(),
            child: None,
            running_flag: Arc::new(Mutex::new(false)),
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

        self.child.replace(Command::new(program)
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()) // Capture stderr
            .stdin(Stdio::piped())
            .envs(self.env.iter())  // Add environment variables from the HashMap
            .spawn()
            .expect("Failed to start the program"));
        let mut child = self.child.take().unwrap();
        set_process_running(&self.running_flag, true);
        println!("Started the program loop {:?}", child);
        
        if self.input.is_some() {
            if let Some(mut stdin) = child.stdin.take() {
                let input = self.input.as_ref().unwrap();
                stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
            }
        }

        let mut stdout = child.stdout.take().expect("Failed to get stdout");
        let running_flag = Arc::clone(&self.running_flag);
        let output_handler = Arc::clone(&self.output_handler);
        thread::spawn(move || process_output("stdout", &mut stdout, running_flag, output_handler));

        let mut stderr = child.stderr.take().expect("Failed to get stderr");
        let running_flag = Arc::clone(&self.running_flag);
        let output_handler = Arc::clone(&self.output_handler);
        thread::spawn(move || process_output("stderr", &mut stderr, running_flag, output_handler));

        let running_flag = Arc::clone(&self.running_flag);
        let status_handler = Arc::clone(&self.status_handler);
        thread::spawn(move || process_status(child, running_flag, status_handler));

        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop_async();
        if let Some(child) = self.child.as_mut() {
            match child.wait() {
                Ok(status) => {
                    println!("Program exited with status: {}", status);
                }
                Err(e) => {
                    eprintln!("Error occurred while waiting for the process: {}", e);
                }
            }
        }
    }

    pub fn stop_async(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.kill() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error occurred while killing the program: {}", e);
                }
            }
        } else { 
            eprintln!("Program exited without a child process"); 
        }
    }
    
    pub fn is_running(&self) -> bool {
        *self.running_flag.lock().unwrap()
    }
    
}

fn process_output(reader_name: &str,
                  reader: &mut dyn Read,
                  running_flag: Arc<Mutex<bool>>,
                  output_handler: Arc<Mutex<dyn FnMut(String) + Send>>) {
    let mut reader = BufReader::new(reader);
    loop {
        println!("Reading from {}...", reader_name);
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer).expect("Failed to read!");
        println!("{}: {}", reader_name, buffer);
        if !buffer.is_empty() {
            let mut handler = output_handler.lock().unwrap();
            (handler)(buffer);
            continue;
        } else if !*running_flag.lock().unwrap() {
            println!("Stopping loop {}", reader_name);
            break;
        }
    }
}

fn process_status(mut child: Child,
                  running_flag: Arc<Mutex<bool>>,
                  status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>) {
    loop {
        println!("Check process status...");
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("Program exited with status: {}", status);
                let mut handler = status_handler.lock().unwrap();
                (handler)(status);
                break;
            }
            Err(e) => {
                eprintln!("Error occurred while waiting for the process: {}", e);
                break;
            }
            Ok(None) => {
                println!("Program is still running...");
                thread::sleep(Duration::from_secs(1)); // Wait for 1 second before checking again
            }
        }
    }
    set_process_running(&running_flag, false);
}

fn set_process_running(mut running_flag: &Arc<Mutex<bool>>, value: bool) {
    *running_flag.lock().expect("Failed to lock process started") = value;
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