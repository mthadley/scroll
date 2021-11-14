mod app;
mod cmd;
mod term;

use std::io::Result;

fn main() -> Result<()> {
    app::run()
}
