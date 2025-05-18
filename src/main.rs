//! # program-tray
//!
//! This UI application can wrap any CLI-program or service in a tray for background work.
//!

mod config;
mod launcher;
mod ui;

use crate::config::Program;
use crate::launcher::Launcher;
use crate::ui::icons::Icons;
use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use log::debug;
use std::cell::RefCell;
use std::rc::Rc;

/// Wrap any CLI-program or service in a tray for background work
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Check config file only
    #[arg(short, long)]
    check_only: bool,

    /// Path to config file
    #[arg(value_name = "PATH")]
    file_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init()?;

    println!("Loading config file: '{}'", args.file_path);
    let program = config::parse_properties_file(&args.file_path)?;
    println!("Found program '{}'", program.get_id());

    let icons = ui::icons::load_icons(&program)?;

    let launcher = Rc::new(RefCell::new(Launcher::new(&program)));

    if args.check_only {
        println!("Check completed")
    } else {
        run_ui(&program, &icons, &launcher)?
    }

    stop_if_running(&launcher)?;
    Ok(())
}

fn run_ui(program: &Program, icons: &Icons, launcher: &Rc<RefCell<Launcher>>) -> Result<()> {
    debug!("Running UI");
    gtk::init()?;

    debug!("Initializing program tray");
    let mut app = ui::app::App::new(&program, &icons, &launcher);
    app.start();

    debug!("UI started");
    gtk::main();

    debug!("Quitting...");
    Ok(())
}

fn stop_if_running(launcher: &Rc<RefCell<Launcher>>) -> Result<()> {
    let mut launcher = launcher.borrow_mut();
    if launcher.is_running() {
        println!("Shutting down running program");
        launcher.stop()?;
    }

    Ok(())
}
