mod tui;
use crate::tui::Tui;
use ratatui::prelude::CrosstermBackend;
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut tui = Tui::new(backend)?;

    tui.setup()?;

    loop {
        tui.draw()?;
        if !tui.handle_input()? {
            break;
        }
    }

    tui.teardown()?;

    Ok(())
}
