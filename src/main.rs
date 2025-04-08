use gtk::prelude::*;
use muda::{MenuItem};
use tray_icon::{TrayIconBuilder, TrayIconEvent, Icon, menu::{Menu, MenuEvent}};

const TITLE: &str = "VPN TRAY";
const ICON_ON: &[u8] = include_bytes!("../resources/on.png");
const ICON_OFF: &[u8] = include_bytes!("../resources/off.png");

fn load_embedded_icon(data: &[u8]) -> Icon {
    let img = image::load_from_memory(data)
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
    
    // Build the tray icon
    let tray_menu = Menu::new();
    let item_run = MenuItem::new("Connect", true, None);
    let item_quit = MenuItem::new("Quit", true, None);

    println!("item_quit: {:?}", item_quit.id());
    tray_menu.append(&item_run).unwrap();
    tray_menu.append(&item_quit).unwrap();
    println!("tray_menu: {:?}", tray_menu.id());
    let tray_icon = TrayIconBuilder::new()
        .with_icon(load_embedded_icon(ICON_OFF))
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
           // tray_icon.set_icon(Some(load_embedded_icon(ICON_OFF)))
              //  .expect("Failed to load embedded icon");
/*
            let item_run_id = item_run.id();
            let item_quit_id = item_quit.id();
            if (id == item_run_id) {
                tray_icon.set_icon(Some(load_embedded_icon(ICON_OFF)))
                    .expect("Failed to load embedded icon");
            } else if (id == item_quit_id) {
                gtk::main_quit();
            }
 */
        }
    }));

    // Start the GTK main loop
    gtk::main();
}