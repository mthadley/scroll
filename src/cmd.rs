use crate::mode::Mode;
use termion::event::Key;

pub enum Cmd {
    View(ViewCmd),
    Search(SearchCmd),
}

impl Cmd {
    pub fn from_key(mode: &Mode, key: Key) -> Self {
        match mode {
            Mode::Viewing(_) => Cmd::View(key.into()),
            Mode::Searching(_) => Cmd::Search(key.into()),
        }
    }
}

pub enum ViewCmd {
    Quit,
    Scroll(Dir),
    StartSearching,
    NextSearchResult,
    Noop,
}

#[derive(Clone, Copy)]
pub enum Dir {
    Up(usize),
    Down(usize),
    HalfPageUp,
    HalfPageDown,
    Top,
    Bottom,
}

impl From<Key> for ViewCmd {
    fn from(key: Key) -> Self {
        match key {
            Key::Char('q') | Key::Ctrl('c') => ViewCmd::Quit,
            Key::Char('j') | Key::Down => ViewCmd::Scroll(Dir::Down(1)),
            Key::Char('k') | Key::Up => ViewCmd::Scroll(Dir::Up(1)),
            Key::Char('n') => ViewCmd::NextSearchResult,
            Key::Char('g') | Key::Home => ViewCmd::Scroll(Dir::Top),
            Key::Char('G') | Key::End => ViewCmd::Scroll(Dir::Bottom),
            Key::Ctrl('d') | Key::PageDown => ViewCmd::Scroll(Dir::HalfPageDown),
            Key::Ctrl('u') | Key::PageUp => ViewCmd::Scroll(Dir::HalfPageUp),
            Key::Char('/') => ViewCmd::StartSearching,
            _ => ViewCmd::Noop,
        }
    }
}

pub enum SearchCmd {
    EnterText(String),
    RemoveChar,
    Confirm,
    Noop,
}

impl From<Key> for SearchCmd {
    fn from(key: Key) -> Self {
        match key {
            Key::Char('\n') => SearchCmd::Confirm,
            Key::Char(char) => SearchCmd::EnterText(char.into()),
            Key::Backspace => SearchCmd::RemoveChar,
            _ => SearchCmd::Noop,
        }
    }
}
