use crate::config::Program;
use shlex::split;
use std::collections::HashMap;
use std::io::{Read, Result, Write};
use std::ops::{Deref, DerefMut};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};

pub struct Launcher {
    command: String,
    superuser: bool,
    input: Option<String>,
    env: HashMap<String, String>,
    child: Arc<Mutex<Option<Child>>>,
    output_handler: Arc<Mutex<dyn FnMut(String) + Send>>,
    status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>,
}

impl Launcher {
    
    pub fn new(program: &Program) -> Self {
        Launcher {
            command: program.command().clone(),
            superuser: true,
            input: program.input().clone(),
            env: program.env().clone(),
            child: Arc::new(Mutex::new(None)),
            output_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
            status_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
        }
    }

    #[cfg(test)]
    pub fn test_new(command: String, env: HashMap<String, String>) -> Self {
        Launcher {
            command: command.clone(),
            superuser: false,
            input: None,
            env: env.clone(),
            child: Arc::new(Mutex::new(None)),
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
            .stdin(Stdio::piped())
            .envs(self.env.iter())  // Add environment variables from the HashMap
            .spawn()
            .expect("Failed to start the program");

        if self.input.is_some() {
            if let Some(mut stdin) = child.stdin.take() {
                let input = self.input.as_ref().unwrap();
                stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
            }
        }

        let mut stdout = child.stdout.take().expect("Failed to get stdout");
        setup_unblocking(&stdout);
        let mut stderr = child.stderr.take().expect("Failed to get stderr");
        setup_unblocking(&stderr);

        println!("Starting the program loop {:?}", child);
        keep_child(&self.child, child);

        let output_handler = Arc::clone(&self.output_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_output("stdout", &mut stdout, &child, output_handler));

        let output_handler = Arc::clone(&self.output_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_output("stderr", &mut stderr, &child, output_handler));

        let status_handler = Arc::clone(&self.status_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_status(&child, status_handler));

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        stop(&self.child, self.superuser, false)
    }

    pub fn stop_async(&mut self) {
        let child = Arc::clone(&self.child);
        let is_superuser = self.superuser;
        thread::spawn(move || stop(&child, is_superuser,true));
    }
    
    pub fn is_running(&self) -> bool {
        is_running(&self.child)
    }
    
}

fn setup_unblocking(output: &dyn AsRawFd) {
    let fd = output.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(output.as_raw_fd(), libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

fn process_output(reader_name: &str,
                  reader: &mut dyn Read,
                  child: &Arc<Mutex<Option<Child>>>,
                  output_handler: Arc<Mutex<dyn FnMut(String) + Send>>) {
    let mut buf = [0u8; 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                println!("Read {} bytes from {}", buf.len(), reader_name);
            },
            Ok(n) => {
                let str = String::from_utf8_lossy(&buf[..n]);
                println!("{}: {}", reader_name, str);
                let mut handler = output_handler.lock().unwrap();
                (handler)(str.to_string());
                continue;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                eprintln!("Error occurred while reading {}: {}", reader_name, e);
            },
        }
        if !is_running(&child) {
            println!("Stopping loop {}", reader_name);
            break;
        }
    }
}

fn process_status(mut child: &Arc<Mutex<Option<Child>>>,
                  status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>) {
    loop {
        println!("Check process status...");
        match wait_child(child) {
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
    forget_child(child);
}

fn keep_child(state: &Arc<Mutex<Option<Child>>>, new_child: Child) {
    let mut locked = state.lock().unwrap(); // or handle the error properly
    *locked = Some(new_child);
}

fn forget_child(state: &Arc<Mutex<Option<Child>>>) {
    let mut locked = state.lock().unwrap();
    *locked = None;
}

fn wait_child(state: &Arc<Mutex<Option<Child>>>) -> Result<Option<ExitStatus>> {
    let mut locked = state.lock().unwrap();
    if let Some(mut child) = locked.as_mut() {
        child.try_wait()
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, "No child is running"))
    }
}

fn is_running(state: &Arc<Mutex<Option<Child>>>) -> bool {
    let locked = state.lock().unwrap();
    match locked.as_ref() {
        None => false,
        Some(_) => true,
    }
}

pub fn stop(state: &Arc<Mutex<Option<Child>>>, is_superuser: bool, is_async: bool) -> Result<()> {
    let mut locked = state.lock().unwrap();
    let child = locked.as_mut()
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "no child pid"))?;

    let pid = child.id().to_string();
    let status = if is_superuser {
        Command::new("pkexec")
            .arg("kill")
            .arg("-INT")
            .arg(pid)
            .status()?
    } else {
        Command::new("kill")
            .arg("-INT")
            .arg(pid)
            .status()?
    };
    
    match status.code() {
        Some(0) => {
            Ok(())
        }
        Some(code) => {
            let msg = format!("Kill command failed with status {}", code);
            Err(io::Error::new(io::ErrorKind::Other, msg))
        }
        _ => Err(io::Error::new(io::ErrorKind::Other, "Kill command failed")),
    }?;

    if is_async {
        return Ok(())
    }
    
    match child.wait() {
        Ok(_) => {
            println!("Stopped gracefully");
            *locked = None;
            Ok(())
        }
        Err(e) => {
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::launcher::Launcher;
    use std::collections::HashMap;
    use std::io::Write;
    use std::option::Option;
    use std::process::ExitStatus;
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use tempfile::NamedTempFile;

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
                match locked.as_mut() {
                    None => *locked = Some(str.clone()),
                    Some(existing) => existing.push_str(&str),
                }
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

    #[test]
    fn stop_process() {
        let temp_file = NamedTempFile::new().unwrap();

        temp_file.as_file().write_all(br#"
          while true; do
            sleep 1
          done
        "#).unwrap();
        
        let path = temp_file.path().to_str().unwrap();
        let cmd = format!("sh {}", path);
        let mut launcher = Launcher::test_new(cmd, HashMap::new());
        
        launcher.start().unwrap();
        
        assert!(launcher.is_running());
        
        launcher.stop().unwrap();

        assert!(!launcher.is_running());
    }

    #[test]
    fn stop_process_async() {
        let temp_file = NamedTempFile::new().unwrap();

        temp_file.as_file().write_all(br#"
          while true; do
            sleep 1
          done
        "#).unwrap();

        let path = temp_file.path().to_str().unwrap();
        let cmd = format!("sh {}", path);
        let mut launcher = Launcher::test_new(cmd, HashMap::new());

        launcher.start().unwrap();

        assert!(launcher.is_running());

        launcher.stop_async();

        await_condition(move || !launcher.is_running());
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