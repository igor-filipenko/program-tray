use crate::config::Program;
use crate::ui::component::{Component, MenuAction, Message, TerminalAction};
use gtk::glib::{Propagation, Sender};
use gtk::prelude::*;
use gtk::{Button, ButtonsType, DialogFlags, MessageType, TextBuffer, TextView, Window};
use std::process::ExitStatus;

const MARK_END: &str = "end";

#[derive(Clone)]
pub struct Terminal {
    window: Window,
    button: Button,
    buffer: TextBuffer,
    text_view: TextView,
    is_program_running: bool,
}

impl Component for Terminal {
    fn start(&mut self, tx: &Sender<Message>) {
        self.connect_delete_event();
        self.connect_close_event(tx);
    }

    fn on_message_received(&mut self, msg: &Message) {
        match msg {
            Message::TrayMenu(action) => self.on_tray_menu_selected(action),
            Message::ProgramStopped(status) => self.on_program_stopped(status),
            Message::ProgramOutput(text) => self.add_string(text),
            Message::Terminal(_) => {}
        }
    }
}

impl Terminal {
    pub fn new(program: &Program) -> Terminal {
        // Create the main window (hidden by default)
        let window = Window::new(gtk::WindowType::Toplevel);
        window.set_title(program.get_title());
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
        let button = Button::with_label("Close");
        button.set_margin_start(10);
        button.set_margin_end(10);
        button.set_margin_top(5);
        button.set_margin_bottom(5);
        button.set_halign(gtk::Align::End);

        // Add widgets to the vertical box
        vbox.pack_start(&scrolled_window, true, true, 0); // Expand Terminal
        vbox.pack_start(&button, false, false, 0); // Place button at the bottom

        // Add the vertical box to the main window
        window.add(&vbox);

        let buffer = text_view.buffer().expect("Failed to get buffer");
        let end_iter = buffer.end_iter();
        buffer.create_mark(Some(MARK_END), &end_iter, false);

        Self {
            window,
            button,
            buffer,
            text_view,
            is_program_running: false,
        }
    }

    fn on_tray_menu_selected(&mut self, action: &MenuAction) {
        match action {
            MenuAction::RUN => {
                if !self.is_program_running {
                    self.clear();
                    self.is_program_running = true;
                }
            }
            MenuAction::VISIBILITY => {
                if self.window.get_visible() {
                    self.window.hide();
                } else {
                    self.window.show_all();
                }
            }
            _ => {}
        }
    }

    fn on_program_stopped(&mut self, status: &ExitStatus) {
        let msg = format!("Program stopped with status {}", status);
        self.add_string(&msg.to_string());
        self.is_program_running = false;
    }

    pub fn add_string(&self, str: &String) {
        let mut end = self.buffer.end_iter();
        self.buffer.insert(&mut end, &str);
        self.buffer.move_mark_by_name(MARK_END, &end);
        let mark = &self
            .buffer
            .mark(MARK_END)
            .expect("No mark {MARK_END} found");
        self.text_view.scroll_to_mark(mark, 0.0, false, 0.0, 0.0);
    }

    pub fn clear(&self) {
        self.buffer.set_text("");
    }

    fn connect_delete_event(&self) {
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

    fn connect_close_event(&self, tx: &Sender<Message>) {
        let window = self.window.clone();
        let tx = tx.clone();
        self.button.connect_clicked(move |_| {
            window.hide();
            let _ = tx.send(Message::Terminal(TerminalAction::HIDE));
        });
    }
}
