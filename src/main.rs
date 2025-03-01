use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::io;

// Tui struct represents the terminal user interface, parameterized by the type of backend (B)
struct Tui<B: Backend> {
    terminal: Terminal<B>,
}

impl<B: Backend> Tui<B> {
    // Constructor method to create a new Tui instance with the provided backend
    fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    // Setup method to initialize raw mode and the alternate screen buffer
    fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?; // Enable raw mode so we can read user input directly
        let mut stdout = io::stdout(); // Get a handle to the standard output stream
        execute!(stdout, EnterAlternateScreen)?; // Switch to an alternate screen buffer (for TUI)
        Ok(())
    }

    // Teardown method to clean up after the program finishes
    fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?; // Disable raw mode before exiting
        let mut stdout = io::stdout(); // Get a handle to stdout again
        execute!(stdout, LeaveAlternateScreen)?; // Leave the alternate screen buffer and return to the normal terminal screen
        self.terminal.show_cursor()?; // Show the cursor again after exiting
        Ok(())
    }

    // Method to draw the user interface on the terminal screen
    fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            // Draw UI using the terminal's draw method
            let size = f.area();

            // cool title
            let ascii_art = [
                " █████╗       ██████╗ ██╗████████╗   ██████╗ ███████╗ █████╗ ████████╗███████╗",
                "██╔══██╗      ██╔══██╗██║╚══██╔══╝   ██╔══██╗██╔════╝██╔══██╗╚══██╔══╝██╔════╝",
                "╚█████╔╝█████╗██████╔╝██║   ██║█████╗██████╔╝█████╗  ███████║   ██║   ███████╗",
                "██╔══██╗╚════╝██╔══██╗██║   ██║╚════╝██╔══██╗██╔══╝  ██╔══██║   ██║   ╚════██║",
                "╚█████╔╝      ██████╔╝██║   ██║      ██████╔╝███████╗██║  ██║   ██║   ███████║",
                " ╚════╝       ╚═════╝ ╚═╝   ╚═╝      ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚══════╝",
                "                                                                              ",
                "                       ♪ ♫ ♪  The 8 Bit Music DJ  ♪ ♫ ♪                       ",
            ];

            // convert each line of the ASCII art into a styled paragraph, then center-align it
            let title = Paragraph::new(
                ascii_art
                    .iter()
                    .map(|&line| Line::from(Span::styled(line, Style::default().fg(Color::Blue))))
                    .collect::<Vec<_>>(),
            )
            .alignment(Alignment::Center);

            // Create a layout for the UI, defining how to divide the terminal area
            let layout = Layout::default()
                .direction(Direction::Vertical) // Stack widgets vertically
                .constraints([Constraint::Percentage(5), Constraint::Percentage(20)].as_ref())
                .split(size);

            // Render the title widget into the second section of the layout (the larger one)
            f.render_widget(title, layout[1]);
        })?;
        Ok(())
    }

    // method to handle user input
    fn handle_input(&self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

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
