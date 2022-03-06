mod app;
mod cmd;
mod mode;
mod term;

use std::io::Result;

fn main() -> Result<()> {
    app::run()
}
