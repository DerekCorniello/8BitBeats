use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Terminal,
};
use std::io;

// TUI struct represents the terminal user interface, parameterized by the type of backend (B)
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
}

impl<B: Backend> Tui<B> {
    // Constructor method to create a new Tui instance with the provided backend
    pub fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    // Setup method to initialize raw mode and the alternate screen buffer
    pub fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?; // Enable raw mode so we can read user input directly
        let mut stdout = io::stdout(); // Get a handle to the standard output stream
        execute!(stdout, EnterAlternateScreen)?; // Switch to an alternate screen buffer (for TUI)
        Ok(())
    }

    // Teardown method to clean up after the program finishes
    pub fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?; // Disable raw mode before exiting
        let mut stdout = io::stdout(); // Get a handle to stdout again
        execute!(stdout, LeaveAlternateScreen)?; // Leave the alternate screen buffer and return to the normal terminal screen
        self.terminal.show_cursor()?; // Show the cursor again after exiting
        Ok(())
    }

    // Method to draw the user interface on the terminal screen

    pub fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            // Get the full area of the terminal.
            let size = f.area();
            let terminal_width = size.width;
            let terminal_height = size.height;

            // Define a layout area with a small padding.
            let layout_width = terminal_width.saturating_sub(2);
            let layout_height = terminal_height.saturating_sub(2);
            let x_offset = (terminal_width - layout_width) / 2;
            let y_offset = (terminal_height - layout_height) / 2;

            // Define your ASCII title.
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

            // Convert each title line into a styled Line.
            let title_lines: Vec<Line> = ascii_art
                .iter()
                .map(|&line| Line::from(Span::styled(line, Style::default().fg(Color::Blue))))
                .collect();

            // Create the title Paragraph.
            let title_paragraph = Paragraph::new(title_lines.clone()).alignment(Alignment::Center);

            // Compute the maximum content width of the title.
            let title_content_width = title_lines
                .iter()
                .map(|line| line.width())
                .max()
                .unwrap_or(0) as u16;
            // Add minimal horizontal padding (1 cell each side) for the border.
            let block_width = title_content_width + 2;

            // Define the rectangle for the title and center it horizontally.
            let title_area = Rect::new(
                x_offset + (layout_width.saturating_sub(block_width)) / 2,
                y_offset,
                block_width,
                title_lines.len() as u16,
            );

            // Render the title.
            f.render_widget(title_paragraph, title_area);

            // --- Compute the rectangles for the input boxes ---
            // We want the input boxes to have the same fixed width as the title block.
            let input_box_width = block_width;
            // Choose fixed heights for each section (adjust as needed)
            let now_playing_height = 8;
            let create_new_track_height = 9;
            let load_track_height = 6;
            // Set a small gap between boxes.
            let gap = 1;

            // Start the input boxes just below the title (with one gap line).
            let mut current_y = y_offset + title_area.height + gap;
            let input_x = x_offset + (layout_width.saturating_sub(input_box_width)) / 2;

            let now_playing_rect =
                Rect::new(input_x, current_y, input_box_width, now_playing_height);
            current_y += now_playing_height + gap;
            let create_new_track_rect =
                Rect::new(input_x, current_y, input_box_width, create_new_track_height);
            current_y += create_new_track_height + gap;
            let load_track_rect = Rect::new(input_x, current_y, input_box_width, load_track_height);

            // --- Create and render each input widget with a border that only wraps its content ---
            let now_playing_text = "Now Playing: [Generated Track ID] - [01:15 / 02:30]\n\n\
Progress: ██████████████████░░░░░░░░░░░░░░░░░░░░   [50%]\n\n\
[<< Rewind]  [▶ Play/Pause]  [>> Skip]  [↺ Enable Loop]";
            let now_playing_widget = Paragraph::new(now_playing_text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .padding(Padding::ZERO),
                );

            let create_new_track_text = "Create New Track\n\n\
Scale: [ Major    ▼] BPM: [120   ] Length: [30 sec  ]\n\n\
Seed (optional): [______]\n\n\
[♫ Generate]";
            let create_new_track_widget = Paragraph::new(create_new_track_text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .padding(Padding::ZERO),
                );

            let load_track_text = "Load Track by ID\n\n\
Track ID: [________________________________] [↓ Load]";
            let load_track_widget = Paragraph::new(load_track_text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .padding(Padding::ZERO),
                );

            // Render each widget into its respective area.
            f.render_widget(now_playing_widget, now_playing_rect);
            f.render_widget(create_new_track_widget, create_new_track_rect);
            f.render_widget(load_track_widget, load_track_rect);
        })?;
        Ok(())
    }
    // Method to handle user input
    pub fn handle_input(&self) -> io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
