mod app;
mod cmd;
mod source;
mod term;

use std::io::Result;

fn main() -> Result<()> {
    app::run()
}
