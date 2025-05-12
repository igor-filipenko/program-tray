use crate::config::Program;
use crate::launcher::Launcher;
use crate::ui::adapter::LauncherAdapter;
use crate::ui::component::*;
use crate::ui::icons::Icons;
use crate::ui::terminal::Terminal;
use crate::ui::tray::Tray;
use gtk::glib;
use gtk::glib::Priority;
use std::cell::RefCell;
use std::rc::Rc;

/// The structure of UI interface
///
pub struct App {
    tray: Tray,
    terminal: Terminal,
    launcher: LauncherAdapter,
}

impl App {
    pub fn new(program: &Program, icons: &Icons, launcher: &Rc<RefCell<Launcher>>) -> Self {
        let tray = Tray::new(program, icons);
        let terminal = Terminal::new(program);
        let launcher = LauncherAdapter::new(launcher); // wtf???
        Self {
            tray,
            terminal,
            launcher,
        }
    }

    pub fn start(&mut self) {
        let (tx, rx) = glib::MainContext::channel(Priority::DEFAULT);

        let mut handlers: Vec<Box<dyn Component>> = vec![
            Box::new(self.tray.clone()),
            Box::new(self.terminal.clone()),
            Box::new(self.launcher.clone()),
        ];

        handlers.iter_mut().for_each(|h| h.start(&tx));

        rx.attach(None, move |msg| {
            handlers
                .iter_mut()
                .for_each(|h| h.on_message_received(&msg));
            glib::ControlFlow::Continue
        });
    }
}
