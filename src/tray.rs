use gtk::glib;
use gtk::glib::Priority;
use gtk::prelude::*;
use muda::MenuItem;
use std::sync::Arc;
use tray_icon::{menu::{Menu, MenuEvent}, Icon, TrayIcon, TrayIconBuilder};

const TITLE: &str = "No name";
const ICON_ON: &[u8] = include_bytes!("../resources/on.png");
const ICON_OFF: &[u8] = include_bytes!("../resources/off.png");

pub struct Tray {
    window: Arc<gtk::Window>,
    icon: Arc<TrayIcon>,
    item_run: Arc<MenuItem>,
    item_quit: Arc<MenuItem>,
}

impl Tray {

    pub fn new() -> Self {
        // Create the main window (hidden by default)
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(TITLE);
        window.set_default_size(400, 300);

        // Add some content to the window
        let label = gtk::Label::new(Some(TITLE));
        window.add(&label);

        // Build the tray icon
        let tray_menu = Menu::new();
        let item_run = MenuItem::new("Activate", true, None);
        tray_menu.append(&item_run).unwrap();
        let item_quit = MenuItem::new("Quit", true, None);
        tray_menu.append(&item_quit).unwrap();

        let tray_icon = TrayIconBuilder::new()
            .with_icon(load_embedded_icon(ICON_OFF))
            .with_tooltip(TITLE)
            .with_menu(Box::new(tray_menu))
            .build()
            .expect("Failed to create tray icon");

        Self {
            window: Arc::new(window),
            icon: Arc::new(tray_icon),
            item_run: Arc::new(item_run),
            item_quit: Arc::new(item_quit),
        }
    }

    pub fn start(& self) {
        // Set up event handler for menu items
        let rx = MenuEvent::receiver();

        // Channel to communicate between threads
        let (tx, rx_gtk) = glib::MainContext::channel(Priority::DEFAULT);

        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                // Forward events to GTK main thread
                let _ = tx.send(event);
            }
        });

        // Process menu events in GTK main thread
        let cloned_item_run = Arc::clone(&self.item_run);
        let cloned_item_quit = Arc::clone(&self.item_quit);
        let cloned_icon = Arc::clone(&self.icon);
        let cloned_window = Arc::clone(&self.window);

        rx_gtk.attach(None, move |event| {
            match event.id {
                id if id == cloned_item_run.id() => {
                    println!("Option 2 selected");
                    cloned_item_run.set_enabled(false);
                    cloned_icon.set_icon(Some(load_embedded_icon(ICON_ON))).unwrap();
                    cloned_window.set_visible(true);
                },
                id if id == cloned_item_quit.id() => {
                    println!("Quitting...");
                    gtk::main_quit();
                },
                _ => {}
            }
            glib::ControlFlow::Continue
        });
    }

}

fn load_embedded_icon(data: &[u8]) -> Icon {
    let img = image::load_from_memory(data)
        .expect("Failed to load embedded icon");

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height)
        .expect("Failed to create icon from RGBA data")
}
