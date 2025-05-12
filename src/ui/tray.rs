use crate::config::Program;
use crate::ui::icons::Icons;
use crate::ui::component::{MenuAction, Message, Component, TerminalAction};
use gtk::glib::Sender;
use log::{warn};
use muda::{MenuItem};
use tray_icon::{menu::{Menu, MenuEvent}, Icon, TrayIcon, TrayIconBuilder};

#[derive(Clone)]
pub struct Tray {
    internal: TrayIcon,
    icons: Icons,
    item_run: MenuItem,  // start/stop program
    item_show: MenuItem, // show/hide terminal
    item_quit: MenuItem,
    is_running: bool,
    is_shown: bool,
}

impl Component for Tray {
    
    fn start(&mut self, tx: &Sender<Message>) {
        let rx = MenuEvent::receiver();
        let tx = tx.clone();
        let run_id = self.item_run.id().clone();
        let show_id = self.item_show.id().clone();
        let quit_id = self.item_quit.id().clone();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                let action = match event.id {
                    id if id == run_id => MenuAction::RUN,
                    id if id == show_id => MenuAction::VISIBILITY,
                    id if id == quit_id => MenuAction::QUIT,
                    _ => MenuAction::UNKNOWN(event.id),
                };
                let _ = tx.send(Message::TrayMenu(action));
            }
        });
    }

    fn on_message_received(&mut self, msg: &Message) {
        match msg {
            Message::TrayMenu(action) => self.on_action_selected(action),
            Message::Terminal(action) => self.on_terminal_action(action),
            Message::ProgramStopped(_) => self.on_program_stopped(),
            Message::ProgramOutput(_) => {},
        }
    }
    
}

impl Tray {

    pub fn new(program: &Program, icons: &Icons) -> Self {
        let tray_menu = Menu::new();
        let item_run = MenuItem::new("Start", true, None);
        tray_menu.append(&item_run).unwrap();
        let item_show = MenuItem::new("Show", true, None);
        tray_menu.append(&item_show).unwrap();
        let item_quit = MenuItem::new("Quit", true, None);
        tray_menu.append(&item_quit).unwrap();

        let icons = icons.clone();
        let internal = TrayIconBuilder::new()
            .with_icon(icons.off.clone())
            .with_tooltip(program.get_title())
            .with_menu(Box::new(tray_menu))
            .build()
            .expect("Failed to create tray icon");

        Self { internal, icons, item_run, item_show, item_quit, is_running: false, is_shown: false }
    }

    fn on_action_selected(&mut self, action: &MenuAction) {
        match action {
            MenuAction::RUN => self.toggle_running(),
            MenuAction::VISIBILITY => self.toggle_terminal_visibility(),
            MenuAction::QUIT => gtk::main_quit(),
            MenuAction::UNKNOWN(menuId) => warn!("unknown menu action: {:?}", menuId),
        }
    }
    
    fn on_terminal_action(&mut self, action: &TerminalAction) {
        self.switch_terminal_visibility(match action {
            TerminalAction::HIDE => false,
        })
    }

    fn toggle_running(&mut self) {
        if self.is_running {
            self.item_run.set_enabled(false);
            // waiting for program stop...
        } else {
            self.on_program_started();
        }
    }

    fn on_program_started(&mut self) {
        self.item_run.set_text("Stop");
        self.set_icon(&self.icons.on);
        self.is_running = true;
    }

    fn on_program_stopped(&mut self) {
        self.item_run.set_text("Start");
        self.set_icon(&self.icons.off);
        self.is_running = false;
    }
    
    fn set_icon(&self, icon: &Icon) {
        self.internal.set_icon(Some(icon.clone())).unwrap(); // TODO: unwrap
    }
    
    fn toggle_terminal_visibility(&mut self) {
        self.switch_terminal_visibility(!self.is_running)
    }
    
    fn switch_terminal_visibility(&mut self, visible: bool) {
        if visible {
            self.item_show.set_text("Hide");
        } else {
            self.item_show.set_text("Show");
        }
        self.is_shown = visible;
    }
    
}