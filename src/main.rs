extern crate termion;

mod cmd;
mod source;
mod term;
mod app;

use std::io::Result;

fn main() -> Result<()> {
    app::run()
}
