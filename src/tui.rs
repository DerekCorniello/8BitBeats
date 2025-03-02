use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Terminal,
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
                    (Direction::Down, InputId::Bpm),
                ]),
            },
        );

        graph.insert(
            InputId::Skip,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Loop),
                    (Direction::Left, InputId::PlayPause),
                    (Direction::Down, InputId::Length),
                ]),
            },
        );

        graph.insert(
            InputId::Loop,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Rewind),
                    (Direction::Left, InputId::Skip),
                    (Direction::Down, InputId::Length),
                ]),
            },
        );

        graph.insert(
            InputId::Scale,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Rewind),
                    (Direction::Right, InputId::Bpm),
                    (Direction::Down, InputId::Seed),
                ]),
            },
        );

        graph.insert(
            InputId::Bpm,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::PlayPause),
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
                    (Direction::Up, InputId::Skip),
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
                    (Direction::Up, InputId::Bpm),
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
            static MIN_WIDTH: u16 = 80;
            static MIN_HEIGHT: u16 = 25;

            // Get the full area of the terminal
            let size = f.size();
            let terminal_width = size.width;
            let terminal_height = size.height;

            // Check if terminal size meets minimum requirements
            if terminal_width < MIN_WIDTH || terminal_height < MIN_HEIGHT {
                // Render a warning message if terminal is too small
                let warning = format!(
                    "Terminal too small! Minimum size: {}x{}, Current: {}x{}",
                    MIN_WIDTH, MIN_HEIGHT, terminal_width, terminal_height
                );
                let warning_widget = Paragraph::new(warning)
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                f.render_widget(warning_widget, size);
                return;
            }

            // Create main vertical layout
            let main_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(8), // Title
                    Constraint::Min(3),    // Content with min height
                ])
                .split(size);

            let title_area = main_layout[0];
            let content_area = main_layout[1];

            // Define your ASCII title
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

            // Convert each title line into a styled Line
            let title_lines: Vec<Line> = ascii_art
                .iter()
                .map(|&line| Line::from(Span::styled(line, Style::default().fg(Color::Blue))))
                .collect();

            // Create the title Paragraph
            let title_paragraph = Paragraph::new(title_lines).alignment(Alignment::Center);

            // Render the title
            f.render_widget(title_paragraph, title_area);

            // Create centered content area with a percentage of the available space
            let content_width_percentage = 80; // Use 80% of available width
            let content_width = (content_area.width as u32 * content_width_percentage / 100) as u16;

            // Make sure content width doesn't exceed 100 characters for readability
            let content_width = std::cmp::min(content_width, 100);

            // Calculate horizontal padding to center content
            let h_padding = (content_area.width.saturating_sub(content_width)) / 2;

            // Create content area with horizontal padding
            let centered_content_area = Rect {
                x: content_area.x + h_padding,
                y: content_area.y,
                width: content_width,
                height: content_area.height,
            };

            // Split content area vertically
            let panel_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(8), // Now Playing
                    Constraint::Length(1), // Gap
                    Constraint::Length(9), // Create New Track
                    Constraint::Length(1), // Gap
                    Constraint::Length(6), // Load Track (fixed height)
                    Constraint::Min(0),    // Remaining space (empty)
                ])
                .split(centered_content_area);

            let now_playing_area = panel_layout[0];
            let create_track_area = panel_layout[2];
            let load_track_area = panel_layout[4];
            // Create and render the Now Playing panel
            let now_playing_block = Block::default().title("Now Playing").borders(Borders::ALL);

            // Split the now playing area for content
            let inner_now_playing = now_playing_block.inner(now_playing_area);
            f.render_widget(now_playing_block, now_playing_area);

            // Create now playing content with layout
            let now_playing_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(1), // Track info
                    Constraint::Length(1), // Empty space
                    Constraint::Length(1), // Progress bar
                    Constraint::Length(1), // Empty space
                    Constraint::Min(1),    // Controls
                ])
                .split(inner_now_playing);

            // Track info
            let track_info =
                Paragraph::new("Generated Track ID - [01:15 / 02:30]").alignment(Alignment::Center);
            f.render_widget(track_info, now_playing_layout[0]);

            // Progress bar
            let progress_bar = Paragraph::new("██████████████████░░░░░░░░░░░░░░░░░░░░   [50%]")
                .alignment(Alignment::Center);
            f.render_widget(progress_bar, now_playing_layout[2]);

            // Controls layout
            let controls_layout = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(now_playing_layout[4]);

            // Define control buttons with appropriate styling based on focus
            let rewind_style = if self.current_focus == InputId::Rewind {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let play_pause_style = if self.current_focus == InputId::PlayPause {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let skip_style = if self.current_focus == InputId::Skip {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let loop_style = if self.current_focus == InputId::Loop {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Render control buttons
            let rewind = Paragraph::new("[<< Rewind]")
                .style(rewind_style)
                .alignment(Alignment::Center);
            let play_pause = Paragraph::new("[▶ Play/Pause]")
                .style(play_pause_style)
                .alignment(Alignment::Center);
            let skip = Paragraph::new("[>> Skip]")
                .style(skip_style)
                .alignment(Alignment::Center);
            let loop_button = Paragraph::new("[↺ Enable Loop]")
                .style(loop_style)
                .alignment(Alignment::Center);

            f.render_widget(rewind, controls_layout[0]);
            f.render_widget(play_pause, controls_layout[1]);
            f.render_widget(skip, controls_layout[2]);
            f.render_widget(loop_button, controls_layout[3]);

            // Create and render the Create New Track panel
            let create_track_block = Block::default()
                .title("Create New Track")
                .borders(Borders::ALL);

            let inner_create_track = create_track_block.inner(create_track_area);
            f.render_widget(create_track_block, create_track_area);

            // Create a layout for the create track panel
            let create_track_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Scale, BPM, Length row
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Seed row
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Generate button
                ])
                .split(inner_create_track);

            // Parameters layout (Scale, BPM, Length)
            let params_layout = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(create_track_layout[1]);

            // Style each parameter based on focus
            let scale_style = if self.current_focus == InputId::Scale {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let bpm_style = if self.current_focus == InputId::Bpm {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let length_style = if self.current_focus == InputId::Length {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Render parameters
            let scale = Paragraph::new("Scale: [ Major    ▼]")
                .style(scale_style)
                .alignment(Alignment::Center);
            let bpm = Paragraph::new("BPM: [120   ]")
                .style(bpm_style)
                .alignment(Alignment::Center);
            let length = Paragraph::new("Length: [30 sec  ]")
                .style(length_style)
                .alignment(Alignment::Center);

            f.render_widget(scale, params_layout[0]);
            f.render_widget(bpm, params_layout[1]);
            f.render_widget(length, params_layout[2]);

            // Seed input
            let seed_style = if self.current_focus == InputId::Seed {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let seed = Paragraph::new("Seed (optional): [______]")
                .style(seed_style)
                .alignment(Alignment::Center);
            f.render_widget(seed, create_track_layout[3]);

            // Generate button
            let generate_style = if self.current_focus == InputId::Generate {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let generate = Paragraph::new("[♫ Generate]")
                .style(generate_style)
                .alignment(Alignment::Center);
            f.render_widget(generate, create_track_layout[5]);

            // Create and render the Load Track panel
            let load_track_block = Block::default()
                .title("Load Track by ID")
                .borders(Borders::ALL);

            let inner_load_track = load_track_block.inner(load_track_area);
            f.render_widget(load_track_block, load_track_area);

            // Layout for the load track panel
            let load_track_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(2), // Empty space
                    Constraint::Min(1),    // Track ID and Load button
                ])
                .split(inner_load_track);

            // Layout for Track ID and Load button
            let track_id_layout = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(3, 4), // Track ID input
                    Constraint::Ratio(1, 4), // Load button
                ])
                .split(load_track_layout[1]);

            // Style track ID and load button based on focus
            let track_id_style = if self.current_focus == InputId::TrackID {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let load_style = if self.current_focus == InputId::Load {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Render Track ID and Load button
            let track_id = Paragraph::new("Track ID: [________________________________]")
                .style(track_id_style)
                .alignment(Alignment::Center);
            let load = Paragraph::new("[↓ Load]")
                .style(load_style)
                .alignment(Alignment::Center);

            f.render_widget(track_id, track_id_layout[0]);
            f.render_widget(load, track_id_layout[1]);
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
                        // Handle the Enter key based on current focus
                        // You can expand this to handle different actions for different input focuses
                        todo!("handle input");
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

    // Method to move the focus
    fn move_focus(&mut self, direction: Direction) {
        let current_focus = self.current_focus;
        self.current_focus = next_focus(current_focus, direction);
        self.draw().unwrap();
    }
}

