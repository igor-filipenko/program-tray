mod tray;

use gtk::prelude::*;

fn main() {
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        std::process::exit(1);
    }

    let tray = tray::Tray::new();
    tray.activate();

    // Start the GTK main loop
    gtk::main();
}