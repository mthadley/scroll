extern crate termion;

mod source;

use source::get_source;
use std::cmp::min;
use std::fmt::Display;
use std::io::{self, BufRead, Result, Write};
use std::sync::mpsc::channel;
use std::{thread, time};
use termion::{clear, cursor, get_tty, terminal_size, color::{self, Bg, Fg}, event::Key,
              input::TermRead, raw::{IntoRawMode, RawTerminal}, screen::AlternateScreen};

fn main() -> Result<()> {
    let source = get_source()?.lines();
    let mut state = State::new()?;
    let tty = get_tty()?;

    let (data_tx, data_rx) = channel();
    let (key_tx, key_rx) = channel();

    thread::spawn(move || {
        for r in source {
            data_tx.send(r).expect("Channel has hung up.");
        }
    });

    thread::spawn(move || {
        for key in tty.keys() {
            key_tx.send(key).expect("Channel has hung up.");
        }
    });

    'main_loop: loop {
        for key in key_rx.try_iter() {
            match key? {
                Key::Char('q') | Key::Ctrl('c') => break 'main_loop,
                Key::Char('j') => state.scroll_down(),
                Key::Char('k') => state.scroll_up(),
                Key::Ctrl('d') => state.scroll_half_down(),
                Key::Ctrl('u') => state.scroll_half_up(),
                _ => {}
            }
        }

        for data in data_rx.try_iter() {
            state.append(data?);
        }

        state.draw()?;
        thread::sleep(time::Duration::from_millis(25));
    }

    Ok(())
}

const STATUS_BAR_HEIGHT: u16 = 1;

struct State {
    data: Vec<String>,
    offset: usize,
    term: Term,
    dimensions: (u16, u16),
    dirty: bool,
}

impl State {
    fn new() -> Result<Self> {
        let mut state = State {
            data: Vec::with_capacity(256),
            dimensions: terminal_size()?,
            offset: 0,
            term: Term::new()?,
            dirty: true,
        };

        state.term.hide_cursor()?;
        state.draw()?;

        Ok(state)
    }

    fn draw(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;

        self.term.clear()?;
        self.draw_text()?;
        self.draw_status_bar()?;

        self.term.flush()
    }

    fn draw_status_bar(&mut self) -> Result<()> {
        let len = self.data.len();
        let percent: f32 = (self.offset as f32) / (len as f32) * 100_f32;
        let (term_width, _) = self.dimensions;

        let status = format!(
            "{fg}{bg}{msg:width$}{reset_fg}{reset_bg}",
            msg = format!(" {:.0}% of {} lines.", percent, len),
            width = term_width as usize,
            fg = Fg(color::White),
            bg = Bg(color::LightBlack),
            reset_fg = Fg(color::Reset),
            reset_bg = Bg(color::Reset)
        );

        self.term.write(status)
    }

    fn draw_text(&mut self) -> Result<()> {
        self.term.move_cursor(1, 1)?;

        let height = self.dimensions.1 - STATUS_BAR_HEIGHT;
        let mut line_count = 0;

        for line in self.data.iter().skip(self.offset) {
            if line_count >= height {
                break;
            }

            self.term.write(line)?;
            self.term.write("\n\r")?;

            line_count += 1;
        }

        // Fill in empty lines.
        for _ in 0..height - line_count {
            self.term
                .write(format!("{}~\n\r{}", Fg(color::LightBlack), Fg(color::Reset)))?;
        }

        Ok(())
    }

    fn scroll_down(&mut self) {
        let offset = min(self.offset + 1, self.data.len());
        self.update_offset(offset);
    }

    fn scroll_up(&mut self) {
        let offset = self.offset.checked_sub(1).unwrap_or(0);
        self.update_offset(offset);
    }

    fn scroll_half_up(&mut self) {
        let (_, term_height) = self.dimensions;
        let offset = self.offset
            .checked_sub(term_height as usize / 2)
            .unwrap_or(0);
        self.update_offset(offset);
    }

    fn scroll_half_down(&mut self) {
        let (_, term_height) = self.dimensions;
        let offset = min(self.offset + (term_height as usize) / 2, self.data.len());
        self.update_offset(offset);
    }

    fn update_offset(&mut self, offset: usize) {
        if offset != self.offset {
            self.offset = offset;
            self.dirty = true;
        }
    }

    fn append(&mut self, line: String) {
        self.data.push(line);
        self.dirty = true;
    }
}

struct Term(AlternateScreen<RawTerminal<io::Stdout>>);

impl Term {
    fn new() -> Result<Self> {
        io::stdout()
            .into_raw_mode()
            .map(|t| Term(AlternateScreen::from(t)))
    }

    fn clear(&mut self) -> Result<()> {
        self.write(clear::All)
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }

    fn hide_cursor(&mut self) -> Result<()> {
        self.write(cursor::Hide)
    }

    fn move_cursor(&mut self, x: u16, y: u16) -> Result<()> {
        self.write(cursor::Goto(x, y))
    }

    fn show_cursor(&mut self) -> Result<()> {
        self.write(cursor::Show)
    }

    fn write(&mut self, d: impl Display) -> Result<()> {
        write!(&mut self.0, "{}", d)
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        self.show_cursor().expect("Failed to show the cursor.");
    }
}
