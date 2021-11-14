use termion::event::Key;

/// Possible commands that can be issued by the user.
pub enum Cmd {
    Quit,
    Scroll(Dir),
    Noop,
}

/// Possible directions and places to scroll to.
pub enum Dir {
    Up(usize),
    Down(usize),
    HalfPageUp,
    HalfPageDown,
    Top,
    Bottom,
}

impl From<Key> for Cmd {
    fn from(key: Key) -> Self {
        match key {
            Key::Char('q') | Key::Ctrl('c') => Cmd::Quit,
            Key::Char('j') | Key::Down => Cmd::Scroll(Dir::Down(1)),
            Key::Char('k') | Key::Up => Cmd::Scroll(Dir::Up(1)),
            Key::Char('g') | Key::Home => Cmd::Scroll(Dir::Top),
            Key::Char('G') | Key::End => Cmd::Scroll(Dir::Bottom),
            Key::Ctrl('d') | Key::PageDown => Cmd::Scroll(Dir::HalfPageDown),
            Key::Ctrl('u') | Key::PageUp => Cmd::Scroll(Dir::HalfPageUp),
            _ => Cmd::Noop,
        }
    }
}
