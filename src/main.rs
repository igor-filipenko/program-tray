//! # program-tray
//!
//! This UI application can wrap any CLI-program or service in a tray for background work.
//!

mod config;
mod launcher;
mod ui;

use crate::launcher::Launcher;
use env_logger::Env;
use gtk::prelude::*;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

fn main() -> io::Result<()> {
    // Retrieve command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: program-tray <config-toml-file-path>");
        std::process::exit(1);
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .try_init()
        .expect("Failed to init logger");

    let filepath = &args[1];
    println!("Loading config file: {}", filepath);
    let program = config::parse_properties_file(filepath)?;
    println!("Using program {program:?}");

    let icons = ui::icons::load_icons(&program)?;

    let launcher = Rc::new(RefCell::new(Launcher::new(&program)));

    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        std::process::exit(1);
    }

    let mut app = ui::app::App::new(&program, &icons, &launcher);
    app.start();

    // Start the GTK main loop
    gtk::main();

    let mut launcher = launcher.borrow_mut();
    if launcher.is_running() {
        println!("Shutting down running program");
        launcher.stop().expect("Program still running");
    }

    Ok(())
}
