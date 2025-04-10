mod tray;
mod config;

use gtk::prelude::*;

fn main() {
    // Step 1: Retrieve command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: program-tray <config-file-path>");
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
    println!("Using program {}", program.to_string());

    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        std::process::exit(1);
    }

    let tray = tray::Tray::new();
    tray.start();

    // Start the GTK main loop
    gtk::main();
}