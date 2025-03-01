use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame, Terminal,
};

use std::sync::OnceLock;
use std::{collections::HashMap, io};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum InputId {
    Rewind,
    PlayPause,
    Skip,
    Loop,
    Scale,
    Bpm,
    Length,
    Seed,
    Generate,
    TrackID,
    Load,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug)]
struct InputNode {
    neighbors: HashMap<Direction, InputId>,
}

static INPUT_GRAPH: OnceLock<HashMap<InputId, InputNode>> = OnceLock::new();

fn get_input_graph() -> &'static HashMap<InputId, InputNode> {
    INPUT_GRAPH.get_or_init(|| {
        let mut graph = HashMap::new();

        graph.insert(
            InputId::Rewind,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::PlayPause),
                    (Direction::Left, InputId::Loop),
                    (Direction::Down, InputId::Scale),
                ]),
            },
        );

        graph.insert(
            InputId::PlayPause,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Skip),
                    (Direction::Left, InputId::Rewind),
                    (Direction::Down, InputId::Scale),
                ]),
            },
        );

        graph.insert(
            InputId::Skip,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Loop),
                    (Direction::Left, InputId::PlayPause),
                    (Direction::Down, InputId::Scale),
                ]),
            },
        );

        graph.insert(
            InputId::Loop,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Rewind),
                    (Direction::Left, InputId::Skip),
                    (Direction::Down, InputId::Scale),
                ]),
            },
        );

        graph.insert(
            InputId::Scale,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Bpm),
                    (Direction::Left, InputId::Length),
                    (Direction::Down, InputId::Seed),
                ]),
            },
        );

        graph.insert(
            InputId::Bpm,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Length),
                    (Direction::Left, InputId::Scale),
                    (Direction::Down, InputId::Seed),
                ]),
            },
        );

        graph.insert(
            InputId::Length,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Scale),
                    (Direction::Left, InputId::Bpm),
                    (Direction::Down, InputId::Seed),
                ]),
            },
        );

        graph.insert(
            InputId::Seed,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Scale),
                    (Direction::Down, InputId::Generate),
                ]),
            },
        );

        graph.insert(
            InputId::Generate,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Seed),
                    (Direction::Down, InputId::TrackID),
                ]),
            },
        );

        graph.insert(
            InputId::TrackID,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Generate),
                    (Direction::Left, InputId::Load),
                    (Direction::Right, InputId::Load),
                ]),
            },
        );

        graph.insert(
            InputId::Load,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Generate),
                    (Direction::Left, InputId::TrackID),
                    (Direction::Right, InputId::TrackID),
                ]),
            },
        );
        graph
    })
}

fn next_focus(current: InputId, direction: Direction) -> InputId {
    get_input_graph()
        .get(&current)
        .and_then(|node| node.neighbors.get(&direction).copied())
        .unwrap_or(current) // Return the current focus if no transition is found
}

// TUI struct represents the terminal user interface, parameterized by the type of backend (B)
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    current_focus: InputId,
}

impl<B: Backend> Tui<B> {
    // Constructor method to create a new Tui instance with the provided backend
    pub fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            current_focus: InputId::PlayPause,
        })
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
            println!("{}", size);
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
    pub fn handle_input(&mut self) -> std::io::Result<bool> {
        match event::read()? {
            // Handle key events
            Event::Key(KeyEvent { code, .. }) => {
                match code {
                    KeyCode::Char('q') => {
                        // Quit the app when 'q' is pressed
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        // Handle the Enter key
                        println!("Enter pressed");
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        // Handle Up arrow or vim 'k'
                        self.move_focus(Direction::Up);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        // Handle Down arrow or vim 'j'
                        self.move_focus(Direction::Down);
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        // Handle Left arrow or vim 'h'
                        self.move_focus(Direction::Left);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        // Handle Right arrow or vim 'l'
                        self.move_focus(Direction::Right);
                    }
                    _ => {
                        // For other keys, do nothing or add custom logic
                    }
                }
            }
            // Ignore non-key events (e.g., mouse events, focus events)
            _ => {}
        }
        Ok(true)
    }
    // Render logic based on the current focus
    fn render_focus(&self, f: &mut Frame) {
        let focused_input = self.current_focus;
        let layout_area = f.area();
        // This is just a rough layout, you'll want to calculate these based on your specific UI
        let focused_position = match focused_input {
            InputId::Rewind => Rect::new(1, 2, 12, 1), // Example position
            InputId::PlayPause => Rect::new(14, 2, 12, 1),
            InputId::Skip => Rect::new(27, 2, 12, 1),
            // Add other cases for other inputs
            _ => layout_area,
        };

        let focused_widget = Paragraph::new(Span::styled(
            "[Focus Here]",
            Style::default().fg(Color::Yellow),
        ));

        f.render_widget(focused_widget, focused_position);
    }

    // Method to move the focus
    fn move_focus(&mut self, direction: Direction) {
        let current_focus = self.current_focus;
        self.current_focus = next_focus(current_focus, direction)
    }
}
