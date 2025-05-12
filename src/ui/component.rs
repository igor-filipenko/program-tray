use gtk::glib::Sender;
use muda::MenuId;
use std::process::ExitStatus;

pub enum MenuAction {
    UNKNOWN(MenuId),
    RUN,
    VISIBILITY,
    QUIT,
}

pub enum TerminalAction {
    HIDE,
}

pub enum Message {
    TrayMenu(MenuAction),
    Terminal(TerminalAction),
    ProgramOutput(String),
    ProgramStopped(ExitStatus),
}

pub trait Component {
    fn start(&mut self, tx: &Sender<Message>);

    fn on_message_received(&mut self, msg: &Message);
}
