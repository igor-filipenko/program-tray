mod app;
mod config;
mod launcher;

use std::sync::{Arc, Mutex};
use gtk::prelude::*;
use crate::launcher::Launcher;

fn main() {
    // Retrieve command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: program-tray <config-toml-file-path>");
        std::process::exit(1);
    }

    let filepath = &args[1];
    println!("Loading config file: {}", filepath);
    let program = match config::parse_properties_file(filepath) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("Failed to read {} with error: {}", filepath, msg);
            std::process::exit(1);
        }
    };
    println!("Using program {program:?}");

    let launcher = Arc::new(Mutex::new(Launcher::new(&program)));
    
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        std::process::exit(1);
    }

    let app = app::App::new(&program, &launcher);
    app.start();

    // Start the GTK main loop
    gtk::main();
}