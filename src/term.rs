use std::fmt::Display;
use std::io::{self, Result, Write};
use termion::cursor;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::{clear, terminal_size};

/// Contains the underlying tty, and manages writes to it.
pub struct Term {
    out: AlternateScreen<RawTerminal<io::Stdout>>,
    dimensions: (u16, u16),
}

impl Term {
    /// Returns a new `Term`, putting the current tty into raw mode
    /// at the same time. Also includes a `Drop` implemenation that shows
    /// the cursor again, since otherwise it will stay hidden when the program
    /// exits.
    pub fn new() -> Result<Self> {
        io::stdout().into_raw_mode().and_then(|t| {
            Ok(Term {
                out: AlternateScreen::from(t),
                dimensions: terminal_size()?,
            })
        })
    }

    pub fn clear(&mut self) -> Result<()> {
        self.write(clear::All)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.out.flush()
    }

    pub fn height(&self) -> usize {
        self.dimensions.1 as usize
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        self.write(cursor::Hide)
    }

    pub fn move_cursor(&mut self, x: usize, y: usize) -> Result<()> {
        self.write(cursor::Goto(x as u16, y as u16))
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        self.write(cursor::Show)
    }

    pub fn width(&self) -> usize {
        self.dimensions.0 as usize
    }

    pub fn write(&mut self, d: impl Display) -> Result<()> {
        write!(&mut self.out, "{}", d)
    }

    /// Writes the `string` out to the terminal, and fills in blanks
    /// up to the terminal width.
    pub fn write_line(&mut self, string: &str) -> Result<()> {
        let len = string.len();
        let width = self.width();
        let count = width - (len % width);
        self.write(format!("{}{}\r\n", string, " ".repeat(count)))
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        self.show_cursor().expect("Failed to show the cursor.");
    }
}
