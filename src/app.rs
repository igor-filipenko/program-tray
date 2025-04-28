use std::process::ExitStatus;
use crate::config::Program;
use crate::launcher::Launcher;
use gtk::glib::{Priority, Propagation};
use gtk::prelude::*;
use gtk::{glib, Button, ButtonsType, DialogFlags, MessageType, TextView};
use muda::MenuItem;
use std::sync::{Arc, Mutex};
use tray_icon::{menu::{Menu, MenuEvent}, Icon, TrayIcon, TrayIconBuilder};

const TITLE: &str = "No name";
const ICON_ON: &[u8] = include_bytes!("../resources/on.png");
const ICON_OFF: &[u8] = include_bytes!("../resources/off.png");

/// The structure of UI interface
/// 
pub struct App {
    launcher: Arc<Mutex<Launcher>>,
    terminal: Arc<TextView>,
    window: Arc<gtk::Window>,
    button: Arc<Button>,
    icon: Arc<TrayIcon>,
    item_run: Arc<MenuItem>,
    item_hide: Arc<MenuItem>,
    item_quit: Arc<MenuItem>,
}

enum Message {
    Menu(MenuEvent),
    Output(String),
    Status(ExitStatus),
}

impl App {

    pub fn new(program: &Program, launcher: &Arc<Mutex<Launcher>>) -> Self {
        // Create the main window (hidden by default)
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(program.title());
        window.set_default_size(400, 300);

        // Create a vertical box to organize widgets
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);

        // Create a terminal like widget
        let text_view = TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);

        // Add the terminal to a ScrolledWindow for scrolling
        let scrolled_window = gtk::ScrolledWindow::builder()
            .child(&text_view)
            .visible(true)
            .build();

        // Create a Close Button
        let close_button = Button::with_label("Close");

        // Add widgets to the vertical box
        vbox.pack_start(&scrolled_window, true, true, 0); // Expand Terminal
        vbox.pack_start(&close_button, false, false, 0); // Place button at the bottom

        // Add the vertical box to the main window
        window.add(&vbox);

        // Build the tray icon
        let tray_menu = Menu::new();
        let item_run = MenuItem::new("Start", true, None);
        tray_menu.append(&item_run).unwrap();
        let item_hide = MenuItem::new("Show", true, None);
        tray_menu.append(&item_hide).unwrap();
        let item_quit = MenuItem::new("Quit", true, None);
        tray_menu.append(&item_quit).unwrap();

        let tray_icon = TrayIconBuilder::new()
            .with_icon(load_embedded_icon(ICON_OFF))
            .with_tooltip(TITLE)
            .with_menu(Box::new(tray_menu))
            .build()
            .expect("Failed to create tray icon");

        Self {
            launcher: Arc::clone(launcher),
            terminal: Arc::new(text_view),
            window: Arc::new(window),
            button: Arc::new(close_button),
            icon: Arc::new(tray_icon),
            item_run: Arc::new(item_run),
            item_hide: Arc::new(item_hide),
            item_quit: Arc::new(item_quit),
        }
    }

    pub fn start(&self) {
        // Set up event handler for menu items
        let rx = MenuEvent::receiver();

        // Channel to communicate between threads
        let (tx, rx_gtk) = glib::MainContext::channel(Priority::DEFAULT);

        let cloned_tx = tx.clone();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                // Forward events to GTK main thread
                let _ = cloned_tx.send(Message::Menu(event));
            }
        });

        let mut locked_launcher = self.launcher.lock().unwrap();
        let cloned_tx = tx.clone();
        locked_launcher.set_output_handler(move |str| {
            let _ = cloned_tx.send(Message::Output(str));
        });
        let cloned_tx = tx.clone();
        locked_launcher.set_status_handler(move |status| {
            let _ = cloned_tx.send(Message::Status(status));
        });

        // Process menu events in GTK main thread
        let cloned_launcher = Arc::clone(&self.launcher);
        let cloned_buffer = self.terminal.buffer().unwrap().clone();
        let cloned_item_run = Arc::clone(&self.item_run);
        let cloned_item_hide = Arc::clone(&self.item_hide);
        let cloned_item_quit = Arc::clone(&self.item_quit);
        let cloned_icon = Arc::clone(&self.icon);
        let cloned_window = Arc::clone(&self.window);

        rx_gtk.attach(None, move |event| {
            match event {
                Message::Menu(event) => {
                    match event.id {
                        id if id == cloned_item_run.id() => {
                            let mut locked_launcher = cloned_launcher.lock().unwrap();
                            if locked_launcher.is_running() {
                                cloned_item_run.set_enabled(false);
                                locked_launcher.stop_async();
                            } else {
                                locked_launcher.start().unwrap();
                                cloned_item_run.set_text("Stop");
                                cloned_icon.set_icon(Some(load_embedded_icon(ICON_ON))).unwrap();
                            }
                        },
                        id if id == cloned_item_hide.id() => {
                            if (cloned_window.is_visible()) {
                                cloned_window.hide();
                                cloned_item_hide.set_text("Show");
                            } else {
                                cloned_window.show_all();
                                cloned_item_hide.set_text("Hide");
                            }
                        },
                        id if id == cloned_item_quit.id() => {
                            gtk::main_quit();
                        },
                        _ => {}
                    }
                },
                Message::Output(str) => {
                    let mut end = cloned_buffer.end_iter();
                    cloned_buffer.insert(&mut end, &str);
                }
                Message::Status(exit_status) => {
                    let mut end = cloned_buffer.end_iter();
                    let msg = format!("Program stopped with status {}", exit_status);
                    cloned_buffer.insert(&mut end, &msg);
                    cloned_item_run.set_text("Start");
                    cloned_item_run.set_enabled(true);
                    cloned_icon.set_icon(Some(load_embedded_icon(ICON_OFF))).unwrap();
                }
            }
            glib::ControlFlow::Continue
        });

        self.start_window();
        self.start_button();
    }

    fn start_window(&self) {
        self.window.connect_delete_event(|window, _| {
            // Create a confirmation dialog
            let dialog = gtk::MessageDialog::new(
                Some(window),
                DialogFlags::MODAL,
                MessageType::Question,
                ButtonsType::YesNo,
                "Are you sure you want to quit?",
            );

            // Run the dialog and check the response
            let response = dialog.run();
            dialog.close();

            if response == gtk::ResponseType::Yes {
                gtk::main_quit(); // Terminate the application
                Propagation::Proceed // Allow the window to close
            } else {
                Propagation::Stop // Prevent the window from closing
            }
        });
    }

    fn start_button(&self) {
        let cloned_window = Arc::clone(&self.window);
        let cloned_item_hide = Arc::clone(&self.item_hide);
        self.button.connect_clicked(move |_| {
            cloned_window.hide();
            cloned_item_hide.set_text("Show");
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
