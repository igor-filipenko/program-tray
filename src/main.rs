use gtk::prelude::*;
use muda::{MenuItem};
use tray_icon::{TrayIconBuilder, TrayIconEvent, Icon, menu::{Menu, MenuEvent}};

const TITLE: &str = "VPN TRAY";
const ICON_DATA: &[u8] = include_bytes!("../resources/vpn-off.png");

fn load_embedded_icon() -> Icon {
    let img = image::load_from_memory(ICON_DATA)
        .expect("Failed to load embedded icon");

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height)
        .expect("Failed to create icon from RGBA data")
}

fn main() {
    // Initialize GTK
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK");
        std::process::exit(1);
    }

    // Create the main window (hidden by default)
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(TITLE);
    window.set_default_size(400, 300);

    // Add some content to the window
    let label = gtk::Label::new(Some(TITLE));
    window.add(&label);

    // Load embedded icon
    let icon = load_embedded_icon();
    
    // Build the tray icon
    let tray_menu = Menu::new();
    let item_quit = MenuItem::new("Quit", true, None);
    println!("item_quit: {:?}", item_quit.id());
    tray_menu.append(&item_quit).unwrap();
    println!("tray_menu: {:?}", tray_menu.id());
    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(TITLE)
        .with_menu(Box::new(tray_menu))
        .build()
        .expect("Failed to create tray icon");

    TrayIconEvent::set_event_handler(Some(move | event| {
        println!("fuck");
        if let TrayIconEvent::Click { id, .. } = event {
            println!("click tray: {:?}", id);
        }
    }));
    MenuEvent::set_event_handler(Some(move | event| {
        println!("fuck");
        if let MenuEvent { id, .. } = event {
            println!("click menu: {:?}", id);
            gtk::main_quit();
        }
    }));

    // Start the GTK main loop
    gtk::main();
}