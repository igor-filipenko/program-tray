use gtk::prelude::*;
use muda::MenuItem;
use tray_icon::{menu::{Menu, MenuEvent}, Icon, TrayIconBuilder};

const TITLE: &str = "No name";
const ICON_ON: &[u8] = include_bytes!("../resources/on.png");
const ICON_OFF: &[u8] = include_bytes!("../resources/off.png");

pub(crate) fn run() {
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
    tray_menu.append(&item_run).unwrap();
    let item_quit = MenuItem::new("Quit", true, None);
    tray_menu.append(&item_quit).unwrap();

    let tray_icon = TrayIconBuilder::new()
        .with_icon(load_embedded_icon(ICON_OFF))
        .with_tooltip(TITLE)
        .with_menu(Box::new(tray_menu))
        .build()
        .expect("Failed to create tray icon");

    let menu_run_id = item_run.id().as_ref().parse::<u8>().unwrap();
    let menu_quit_id = item_quit.id().as_ref().parse::<u8>().unwrap();

    MenuEvent::set_event_handler(Some(move | event| {
        if let MenuEvent { id, .. } = event {
            let selected_id = id.as_ref().parse::<u8>().unwrap();
            match selected_id {
                id if id == menu_run_id => {
                    println!("selected to run {selected_id}");
                   // tray_icon.set_icon(Some(load_embedded_icon(ICON_ON))).unwrap();
                }
                id if id == menu_quit_id => {
                    println!("selected to quit {selected_id}");
                    gtk::main_quit()
                }
                _ => eprintln!("Unrecognized menu event")
            }
        }
    }));

    // Start the GTK main loop
    gtk::main();
}

fn load_embedded_icon(data: &[u8]) -> Icon {
    let img = image::load_from_memory(data)
        .expect("Failed to load embedded icon");

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height)
        .expect("Failed to create icon from RGBA data")
}
