//! Contains the main logic for `scroll`.

use cmd::{Cmd, Dir};
use source::get_source;
use std::cmp::{max, min};
use std::io::{BufRead, Result};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use term::Term;
use termion::color::{self, Bg, Fg};
use termion::event::Key;
use termion::get_tty;
use termion::input::TermRead;

pub fn run() -> Result<()> {
    let source = get_source()?.lines();
    let mut state = State::new()?;

    let (tx, rx) = sync_channel(0);

    let key_tx = tx.clone();
    let tty = get_tty()?;
    thread::spawn(move || events_from(tty.keys(), key_tx));

    let data_tx = tx.clone();
    thread::spawn(move || events_from(source, data_tx));

    for event in rx.iter() {
        if state.update(event?)? {
            break;
        }
    }

    Ok(())
}

fn events_from(
    stream: impl Iterator<Item = Result<impl Into<Event>>>,
    tx: SyncSender<Result<Event>>,
) {
    for result in stream {
        tx.send(result.map(|i| i.into()))
            .expect("Channel has hung up.");
    }
}

const STATUS_BAR_HEIGHT: usize = 1;

pub enum Event {
    MoreData(String),
    Command(Cmd),
}

impl From<String> for Event {
    fn from(s: String) -> Self {
        Event::MoreData(s)
    }
}

impl From<Key> for Event {
    fn from(key: Key) -> Self {
        Event::Command(key.into())
    }
}

struct State {
    data: Vec<String>,
    offset: usize,
    term: Term,
    dirty: bool,
}

impl State {
    fn new() -> Result<Self> {
        let mut state = State {
            data: Vec::with_capacity(256),
            offset: 0,
            term: Term::new()?,
            dirty: true,
        };

        state.term.hide_cursor()?;
        state.term.clear()?;
        state.draw()?;

        Ok(state)
    }

    fn update(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::MoreData(data) => self.append(data),
            Event::Command(cmd) => match cmd {
                Cmd::Quit => return Ok(true),
                Cmd::Scroll(dir) => self.scroll(dir),
                Cmd::Noop => {}
            },
        }

        self.draw()?;

        return Ok(false);
    }

    fn draw(&mut self) -> Result<()> {
        if self.dirty {
            self.draw_text()?;
            self.dirty = false;
        }

        self.draw_status_bar()?;

        self.term.flush()
    }

    fn draw_status_bar(&mut self) -> Result<()> {
        let text_height = self.term.height() - (STATUS_BAR_HEIGHT - 1);
        self.term.move_cursor(1, text_height)?;

        let percent: f32 =
            ((self.offset + 1) as f32) / (max(self.max_offset(), 1) as f32) * 100_f32;

        let status = format!(
            "{fg}{bg}{msg:width$}{reset_fg}{reset_bg}",
            msg = format!(" {:.0}% of {} lines.", percent, self.data.len()),
            width = self.term.width(),
            fg = Fg(color::White),
            bg = Bg(color::LightBlack),
            reset_fg = Fg(color::Reset),
            reset_bg = Bg(color::Reset)
        );

        self.term.write(status)
    }

    fn draw_text(&mut self) -> Result<()> {
        self.term.move_cursor(1, 1)?;

        let height = self.term.height() - STATUS_BAR_HEIGHT;
        let mut line_count = 0;

        for line in self.data.iter().skip(self.offset) {
            if line_count >= height {
                break;
            }

            self.term.write_line(line)?;

            line_count += 1;
        }

        // Fill in empty lines.
        for _ in 0..height - line_count {
            self.term.write_line("~")?;
        }

        Ok(())
    }

    fn max_offset(&self) -> usize {
        self.data.len().checked_sub(self.term.height()).unwrap_or(0)
    }

    fn scroll(&mut self, dir: Dir) {
        match dir {
            Dir::Up(count) => self.scroll_up(count),
            Dir::Down(count) => self.scroll_down(count),
            Dir::HalfPageDown => self.scroll_half_down(),
            Dir::HalfPageUp => self.scroll_half_up(),
            Dir::Top => self.update_offset(|_| 0),
            Dir::Bottom => self.scroll_bottom(),
        }
    }

    fn scroll_bottom(&mut self) {
        let offset = self.data.len();
        self.update_offset(|_| offset);
    }

    fn scroll_down(&mut self, count: usize) {
        self.update_offset(|offset| offset + count);
    }

    fn scroll_up(&mut self, count: usize) {
        self.update_offset(|offset| offset.checked_sub(count).unwrap_or(0));
    }

    fn scroll_half_up(&mut self) {
        let height = self.term.height();
        self.update_offset(|offset| offset.checked_sub(height / 2).unwrap_or(0));
    }

    fn scroll_half_down(&mut self) {
        let height = self.term.height();
        self.update_offset(|offset| offset + (height / 2));
    }

    /// Updates the offset, and ensures it stays within the bounds of the screen.
    fn update_offset(&mut self, func: impl FnOnce(usize) -> usize) {
        let offset = min(func(self.offset), self.max_offset());

        if offset != self.offset {
            self.offset = offset;
            self.dirty = true;
        }
    }

    fn append(&mut self, line: String) {
        self.data.push(line);

        if self.data.len() - self.offset <= self.term.height() - STATUS_BAR_HEIGHT {
            self.dirty = true;
        }
    }
}
