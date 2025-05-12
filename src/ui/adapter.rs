use crate::launcher::Launcher;
use crate::ui::component::{Component, MenuAction, Message};
use gtk::glib::Sender;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct LauncherAdapter {
    delegate: Rc<RefCell<Launcher>>,
}

impl LauncherAdapter {
    pub fn new(launcher: &Rc<RefCell<Launcher>>) -> Self {
        Self { delegate: Rc::clone(launcher) }
    }
}

impl Component for LauncherAdapter {

    fn start(&mut self, tx: &Sender<Message>) {
        let mut delegate = self.delegate.borrow_mut();
        let ctx = tx.clone();
        delegate.set_output_handler(move |text| {
            let _ = ctx.send(Message::ProgramOutput(text));
        });
        let ctx = tx.clone();
        delegate.set_status_handler(move |status| {
            let _ = ctx.send(Message::ProgramStopped(status));
        })
    }

    fn on_message_received(&mut self, msg: &Message) {
        match msg {
            Message::TrayMenu(action) => {
                match action {
                    MenuAction::RUN => {
                        let mut launcher = self.delegate.borrow_mut();
                        if !launcher.is_running() {
                            launcher.start().unwrap();
                        } else {
                            launcher.stop_async();
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }

    }

}
