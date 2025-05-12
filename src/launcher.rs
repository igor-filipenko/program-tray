use crate::config::Program;
use log::{debug, error, info, trace};
use shlex::split;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Result, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};

/// Authorise as superuser using UI
const SUDO_COMMAND: &str = "pkexec";

const READER_STDOUT: &str = "stdout";
const READER_STDERR: &str = "stderr";

/// Launch any CLI-program
///
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
            command: program.get_command().clone(),
            superuser: true,
            input: program.get_input().clone(),
            env: program.get_env().clone(),
            child: Arc::new(Mutex::new(None)),
            output_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
            status_handler: Arc::new(Mutex::new(|_| {})), // default empty handler
        }
    }

    #[cfg(test)]
    fn test_new(command: String, env: HashMap<String, String>) -> Self {
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
    
    /// Setup program output handler 
    /// 
    pub fn set_output_handler<F>(&mut self, handler: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        self.output_handler = Arc::new(Mutex::new(handler));
    }

    /// Setup program stopped event handler
    /// 
    pub fn set_status_handler<F>(&mut self, handler: F)
    where
        F: FnMut(ExitStatus) + Send + 'static,
    {
        self.status_handler = Arc::new(Mutex::new(handler));
    }

    /// Start program
    /// 
    pub fn start(&mut self) -> Result<()> {
        if is_running(&self.child) {
            return Err(io::Error::new(ErrorKind::Other, "Already started"))
        }
        
        // Parse the command string into program and arguments
        let parts = split(&self.command).unwrap_or_else(|| vec![self.command.to_string()]);
        if parts.is_empty() {
            return Err(io::Error::new(ErrorKind::InvalidInput, "Empty command string"))
        }

        // Extract the program name and arguments
        let (program, args) = match self.superuser {
            true => (&SUDO_COMMAND.to_string(), &parts[..]),
            false => (&parts[0], &parts[1..]),
        };
        
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()) // Capture stderr
            .stdin(Stdio::piped())
            .envs(self.env.iter())  // Add environment variables from the HashMap
            .spawn()?;

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

        info!("Starting the program loop {:?}", child);
        keep_child(&self.child, child);

        let output_handler = Arc::clone(&self.output_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_output(READER_STDOUT, &mut stdout, &child, output_handler));

        let output_handler = Arc::clone(&self.output_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_output(READER_STDERR, &mut stderr, &child, output_handler));

        let status_handler = Arc::clone(&self.status_handler);
        let child = Arc::clone(&self.child);
        thread::spawn(move || process_status(&child, status_handler));

        Ok(())
    }

    /// Stop the running program.
    /// Blocks the running thread till the program shutdown.
    /// 
    pub fn stop(&mut self) -> Result<()> {
        stop(&self.child, self.superuser, false)
    }

    /// Stop the running program.
    /// No blocking.
    /// 
    pub fn stop_async(&mut self) {
        let child = Arc::clone(&self.child);
        let is_superuser = self.superuser;
        thread::spawn(move || stop(&child, is_superuser,true));
    }
    
    /// Check if the program still running.
    /// 
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
                trace!("Read {} bytes from {}", buf.len(), reader_name);
            },
            Ok(n) => {
                let str = String::from_utf8_lossy(&buf[..n]);
                trace!("{}: {}", reader_name, str);
                let mut handler = output_handler.lock().unwrap();
                (handler)(str.to_string());
                continue;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                error!("Error occurred while reading {}: {}", reader_name, e);
            },
        }
        if !is_running(&child) {
            debug!("Stopping loop {}", reader_name);
            break;
        }
    }
}

fn process_status(child: &Arc<Mutex<Option<Child>>>,
                  status_handler: Arc<Mutex<dyn FnMut(ExitStatus) + Send>>) {
    loop {
        debug!("Check process status...");
        match wait_child(child) {
            Ok(Some(status)) => {
                info!("Program exited with status: {}", status);
                let mut handler = status_handler.lock().unwrap();
                (handler)(status);
                break;
            }
            Err(e) => {
                error!("Error occurred while waiting for the process: {}", e);
                break;
            }
            Ok(None) => {
                trace!("Program is still running...");
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
    if let Some(child) = locked.as_mut() {
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
    if !is_running(state) {
        debug!("Already stopped");
        return Ok(())
    }
    
    let mut locked = state.lock().unwrap();
    let child = locked.as_mut()
        .ok_or(io::Error::new(ErrorKind::NotFound, "no child pid"))?;

    let pid = child.id().to_string();
    let status = if is_superuser {
        Command::new(SUDO_COMMAND)
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
            Err(io::Error::new(ErrorKind::Other, msg))
        }
        _ => Err(io::Error::new(ErrorKind::Other, "Kill command failed")),
    }?;

    if is_async {
        return Ok(())
    }
    
    match child.wait() {
        Ok(_) => {
            debug!("Stopped gracefully");
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
    use env_logger::Env;
    use std::collections::HashMap;
    use std::io::Write;
    use std::option::Option;
    use std::process::ExitStatus;
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use tempfile::NamedTempFile;

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn setup() {
        let _ = env_logger::Builder::from_env(Env::default().default_filter_or("trace"))
            .is_test(true)
            .try_init();
    }
    
    #[test]
    fn execute_echo() {
        setup();
        
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
        let output_clone = Arc::clone(&output);
        await_condition(move || {
            let status_locked = status_clone.lock().unwrap();
            let output_locked = output_clone.lock().unwrap();
            status_locked.is_some() && output_locked.is_some()
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
        setup();
        
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
        let output_clone = Arc::clone(&output);
        await_condition(move || {
            let status_locked = status_clone.lock().unwrap();
            let output_locked = output_clone.lock().unwrap();
            status_locked.is_some() && output_locked.is_some()
        });

        let locked_output = output.lock().unwrap();
        assert!(locked_output.is_some());
        assert!(locked_output.clone().unwrap().lines().any(|line| line.contains("VAR1=VAL1")));
    }

    #[test]
    fn stop_process() {
        setup();
        
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
        setup();
        
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

    #[test]
    fn try_start_process_if_started() {
        setup();

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

        let result = launcher.start();
        assert!(result.is_err());
        
        launcher.stop().unwrap();
    }

    #[test]
    fn blank_command() {
        setup();

        let mut launcher = Launcher::test_new(" ".to_string(), HashMap::new());
        let result = launcher.start();
        assert!(result.is_err());
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
        if !ok {
            panic!("Timed out waiting for condition");
        }
    }
    
}