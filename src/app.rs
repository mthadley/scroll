//! Contains the main logic for `scroll`.

use crate::{
    cmd::{Cmd, Dir, SearchCmd, ViewCmd},
    mode::Mode,
    term::Term,
};
use std::{
    cmp::{max, min},
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    sync::mpsc::{sync_channel, SyncSender},
    thread,
};
use termion::{
    color::{self, Bg, Fg},
    event::Key,
    get_tty,
    input::TermRead,
};

const DATA_BUFFER_SIZE: usize = 500;

pub fn run() -> io::Result<()> {
    let source = get_source()?.lines();
    let mut state = State::new()?;

    let (tx, rx) = sync_channel(0);

    let key_tx = tx.clone();
    let tty = get_tty()?;
    thread::spawn(move || {
        for key in tty.keys() {
            key_tx
                .send(key.map(|i| Event::Key(i)))
                .expect("Channel has hung up.");
        }
    });

    let data_tx = tx.clone();
    thread::spawn(move || {
        let mut lines: Vec<String> = Vec::with_capacity(DATA_BUFFER_SIZE);

        for result in source {
            match result {
                Err(e) => data_tx.send(Err(e)).expect("Channel has hung up."),
                Ok(data) => lines.push(data),
            };

            if lines.len() >= DATA_BUFFER_SIZE {
                send_data(&data_tx, lines);
                lines = Vec::with_capacity(DATA_BUFFER_SIZE);
            }
        }

        send_data(&data_tx, lines);
    });

    for event in rx {
        if state.update(event?)? {
            break;
        }
    }

    Ok(())
}

fn send_data(data_tx: &SyncSender<io::Result<Event>>, data: Vec<String>) {
    data_tx
        .send(Ok(Event::MoreData(data.into_boxed_slice())))
        .expect("Channel has hung up.");
}

/// Attempts to read a file from the passed arguments, or defaults
/// to reading data from stdin.
fn get_source() -> io::Result<Box<dyn BufRead + Send>> {
    if let Some(path) = env::args().nth(1) {
        Ok(Box::new(BufReader::new(File::open(path)?)))
    } else {
        Ok(Box::new(BufReader::new(io::stdin())))
    }
}

const STATUS_BAR_HEIGHT: usize = 1;
const CURSOR_SEARCH_OFFSET: usize = 2;

pub enum Event {
    MoreData(Box<[String]>),
    Key(Key),
}

struct State {
    data: Vec<String>,
    offset: usize,
    term: Term,
    dirty: bool,
    mode: Mode,
}

impl State {
    fn new() -> io::Result<Self> {
        let mut state = State {
            data: Vec::with_capacity(256),
            offset: 0,
            term: Term::new()?,
            dirty: true,
            mode: Mode::Viewing(None),
        };

        state.term.hide_cursor()?;
        state.term.clear()?;
        state.draw()?;

        Ok(state)
    }

    fn update(&mut self, event: Event) -> io::Result<bool> {
        let quit = match event {
            Event::MoreData(data) => {
                self.append(data);
                false
            }
            Event::Key(key) => self.handle_key(key),
        };

        self.draw()?;

        Ok(quit)
    }

    fn handle_key(&mut self, key: Key) -> bool {
        match (&self.mode, &Cmd::from_key(&self.mode, key)) {
            (Mode::Viewing(maybe_search_text), Cmd::View(view_cmd)) => match view_cmd {
                ViewCmd::Quit => return true,
                ViewCmd::Scroll(dir) => self.scroll(*dir),
                ViewCmd::StartSearching => self.mode = Mode::Searching("".into()),
                ViewCmd::NextSearchResult => {
                    if let Some(search_text) = maybe_search_text {
                        self.dirty = true;
                        self.offset = self
                            .next_occurrence_offset(search_text, self.offset + 1)
                            .unwrap_or(self.offset);
                    }
                }
                ViewCmd::Noop => (),
            },
            (Mode::Searching(search_text), Cmd::Search(search_cmd)) => match search_cmd {
                SearchCmd::EnterText(text) => {
                    self.mode = Mode::Searching(search_text.to_owned() + &text)
                }

                SearchCmd::RemoveChar => {
                    if !search_text.is_empty() {
                        self.mode =
                            Mode::Searching(search_text[0..search_text.len() - 1].to_owned())
                    }
                }
                SearchCmd::Confirm => {
                    self.dirty = true;

                    self.mode = Mode::Viewing(if !search_text.is_empty() {
                        self.offset = self
                            .next_occurrence_offset(search_text, self.offset)
                            .unwrap_or(self.offset);
                        Some(search_text.to_owned())
                    } else {
                        None
                    })
                }
                SearchCmd::Noop => (),
            },
            _ => unreachable!("Got mismatched event for current mode."),
        };

        false
    }

    fn draw(&mut self) -> io::Result<()> {
        self.term.hide_cursor()?;

        if self.dirty {
            self.draw_text()?;
            self.dirty = false;
        }

        self.draw_status_bar()?;

        match &self.mode {
            Mode::Viewing(_) => {}
            Mode::Searching(search_text) => {
                self.term
                    .move_cursor(search_text.len() + CURSOR_SEARCH_OFFSET, self.term.height())?;
                self.term.show_cursor()?;
            }
        };

        self.term.flush()
    }

    fn draw_status_bar(&mut self) -> io::Result<()> {
        let text_height = self.term.height() - (STATUS_BAR_HEIGHT - 1);
        self.term.move_cursor(1, text_height)?;

        let msg = match &self.mode {
            Mode::Viewing(_) => {
                let percent: f32 =
                    ((self.offset) as f32) / (max(self.max_offset(), 1) as f32) * 100_f32;
                format!(" {:3.0}% of {} lines", percent, self.data.len())
            }
            Mode::Searching(search_text) => format!("/{}", search_text),
        };

        let status = format!(
            "{fg}{bg}{msg:width$}{reset_fg}{reset_bg}",
            msg = msg,
            width = self.term.width(),
            fg = Fg(color::White),
            bg = Bg(color::LightBlack),
            reset_fg = Fg(color::Reset),
            reset_bg = Bg(color::Reset)
        );

        self.term.write(status)
    }

    fn draw_text(&mut self) -> io::Result<()> {
        self.term.move_cursor(1, 1)?;

        let height = self.term.height() - STATUS_BAR_HEIGHT;
        let mut line_count = 0;

        for line in self.data.iter().skip(self.offset).take(height) {
            if let Mode::Viewing(Some(ref search_text)) = self.mode {
                let highlighted_line = line.replace(
                    search_text,
                    &format!(
                        "{bg}{line}{reset_bg}",
                        line = search_text,
                        bg = Bg(color::LightBlack),
                        reset_bg = Bg(color::Reset)
                    ),
                );

                self.term.write_line(&highlighted_line)?;
            } else {
                self.term.write_line(line)?;
            };

            line_count += 1;
        }

        // Fill in empty lines.
        for _ in 0..height - line_count {
            self.term.write_line("~")?;
        }

        Ok(())
    }

    fn max_offset(&self) -> usize {
        self.data.len().checked_sub(self.term.height()).unwrap_or(0) + STATUS_BAR_HEIGHT
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

    fn next_occurrence_offset(&self, search_text: &str, starting_at: usize) -> Option<usize> {
        self.data
            .iter()
            .skip(starting_at)
            .position(|line| line.contains(search_text))
            .map(|base_offset| base_offset + starting_at)
    }

    fn append(&mut self, lines: Box<[String]>) {
        let old_len = self.data.len();

        self.data.append(&mut Vec::from(lines));

        if old_len + self.offset <= self.term.height() - STATUS_BAR_HEIGHT {
            self.dirty = true;
        }
    }
}
