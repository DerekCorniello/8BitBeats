use crate::gen::{get_music_sender, pause_music, resume_music, start_music_in_thread, stop_music};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::Position,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};

use std::{
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

use std::time::{Duration, Instant};
static IS_PLAYING: AtomicBool = AtomicBool::new(false);
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum InputId {
    Rewind,
    PlayPause,
    Skip,
    Loop,
    Scale,
    Style,
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
                    (Direction::Down, InputId::Style),
                ]),
            },
        );

        graph.insert(
            InputId::Loop,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::Rewind),
                    (Direction::Left, InputId::Skip),
                    (Direction::Down, InputId::Style),
                ]),
            },
        );

        graph.insert(
            InputId::Scale,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Rewind),
                    (Direction::Right, InputId::Style),
                    (Direction::Left, InputId::Style),
                    (Direction::Down, InputId::Bpm),
                ]),
            },
        );

        graph.insert(
            InputId::Style,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Loop),
                    (Direction::Right, InputId::Scale),
                    (Direction::Left, InputId::Scale),
                    (Direction::Down, InputId::Length),
                ]),
            },
        );

        graph.insert(
            InputId::Bpm,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Scale),
                    (Direction::Right, InputId::Length),
                    (Direction::Left, InputId::Length),
                    (Direction::Down, InputId::Seed),
                ]),
            },
        );

        graph.insert(
            InputId::Length,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Style),
                    (Direction::Right, InputId::Bpm),
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
                    (Direction::Left, InputId::Load),
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

// Input mode to determine how to handle user input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Navigation,  // For navigating between fields
    Editing,     // For editing a text field
    ScalePopup,  // When the scale popup is active
    StylePopup,  // When the style popup is active
    LengthPopup, // When the length popup is active
}

// AppState to store application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub scale: String,
    pub style: String,
    pub bpm: String,
    pub progress: f32,
    pub elapsed_play_time: f32,
    pub start_time: Option<Instant>,
    pub length: String,
    pub seed: String,
    pub track_id: String,
    pub input_mode: InputMode,
    pub popup_list_state: ListState,
    pub scales: Vec<String>,
    pub styles: Vec<String>,
    pub lengths: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        // Create list of scales and set initial selection
        let scales = vec![
            "Random".to_string(),
            "C".to_string(),
            "C#".to_string(),
            "D".to_string(),
            "D#".to_string(),
            "E".to_string(),
            "F".to_string(),
            "F#".to_string(),
            "G".to_string(),
            "G#".to_string(),
            "A".to_string(),
            "A#".to_string(),
            "B".to_string(),
        ];

        // Create list of musical styles
        let styles = vec![
            "jazz".to_string(),
            "blues".to_string(),
            "pop".to_string(),
            "basic".to_string(),
        ];

        // Create list of track lengths
        let lengths = vec![
            "1 min".to_string(),
            "2 min".to_string(),
            "5 min".to_string(),
            "10 min".to_string(),
            "20 min".to_string(),
            "30 min".to_string(),
            "60 min".to_string(),
        ];

        let mut popup_list_state = ListState::default();
        popup_list_state.select(Some(0)); // Select the first item by default

        Self {
            scale: "C".to_string(),
            style: "pop".to_string(),
            bpm: "120".to_string(),
            length: "1 min".to_string(),
            seed: "".to_string(),
            track_id: "".to_string(),
            elapsed_play_time: 0.0,
            start_time: Some(Instant::now()),
            input_mode: InputMode::Navigation,
            popup_list_state,
            scales,
            styles,
            lengths,
            progress: 0.0,
        }
    }
}

// TUI struct represents the terminal user interface, parameterized by the type of backend (B)
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    current_focus: InputId,
    state: AppState,
}

impl<B: Backend> Tui<B> {
    // Constructor method to create a new Tui instance with the provided backend
    pub fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let terminal = Terminal::new(backend)?;
        // setup the music process
        Ok(Self {
            terminal,
            current_focus: InputId::PlayPause,
            state: AppState::default(),
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
    // Fix for the draw method - replace the entire draw method with this corrected version
    pub fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            static MIN_WIDTH: u16 = 80;
            static MIN_HEIGHT: u16 = 25;

            // Get the full area of the terminal
            let size = f.area();
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

            // Calculate the total required height for the app
            let title_height = 8; // Title section height
            let content_height = 26; // Content section height (Now Playing + Create New Track + Load Track)
            let total_app_height = title_height + content_height;

            // Calculate vertical padding to center the app
            let v_padding = (terminal_height.saturating_sub(total_app_height)) / 2;

            // Create top-level layout with vertical padding
            let main_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(v_padding),      // Top padding
                    Constraint::Length(title_height),   // Title
                    Constraint::Length(content_height), // Content
                    Constraint::Min(v_padding),         // Bottom padding
                ])
                .split(size);

            let title_area = main_layout[1];
            let content_area = main_layout[2];

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
            let title_paragraph = Paragraph::new(title_lines)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            // Render the title
            f.render_widget(title_paragraph, title_area);

            // Create centered content area with a percentage of the available width
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
                    Constraint::Min(1),    // Remaining space (empty)
                ])
                .split(centered_content_area);

            // Rest of the function remains the same...
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
            let track_info = Paragraph::new("Generated Track ID - [01:15 / 02:30]")
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);
            f.render_widget(track_info, now_playing_layout[0]);

            // Progress bar
            let length = self
                .state
                .length
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<f32>()
                .unwrap();

            let progress_bar = Paragraph::new("██████████████████░░░░░░░░░░░░░░░░░░░░   [50%]")
                .alignment(Alignment::Center);
            f.render_widget(progress_bar, now_playing_layout[2]);

            // Controls layout for Now Playing section
            let control_layout = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(now_playing_layout[4]);

            // Render playback controls
            let rewind_style = if self.current_focus == InputId::Rewind
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let play_pause_style = if self.current_focus == InputId::PlayPause
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let skip_style = if self.current_focus == InputId::Skip
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let loop_style = if self.current_focus == InputId::Loop
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let rewind = Paragraph::new("[<< Rewind]")
                .style(rewind_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            let play_pause = Paragraph::new("[▶ Play/Pause]")
                .style(play_pause_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            let skip = Paragraph::new("[>> Skip]")
                .style(skip_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            let loop_control = Paragraph::new("[↻ Enable Loop]")
                .style(loop_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            f.render_widget(rewind, control_layout[0]);
            f.render_widget(play_pause, control_layout[1]);
            f.render_widget(skip, control_layout[2]);
            f.render_widget(loop_control, control_layout[3]);

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
                    Constraint::Length(1), // Parameters row
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Parameters row
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Seed row
                    Constraint::Length(2), // Empty space
                    Constraint::Length(1), // Generate button
                ])
                .split(inner_create_track);

            // Parameters layout (Scale, Style, BPM, Length)
            let params_layout_top = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(create_track_layout[1]);

            let params_layout_bottom = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(create_track_layout[3]);

            // Style each parameter based on focus and input mode
            let scale_style = if self.current_focus == InputId::Scale {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let style_style = if self.current_focus == InputId::Style {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let bpm_style = if self.current_focus == InputId::Bpm {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let length_style = if self.current_focus == InputId::Length {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            // Render parameters with actual values from state
            let scale = Paragraph::new(format!("Scale: [ {} ▼]", self.state.scale))
                .style(scale_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            let style_param = Paragraph::new(format!("Style: [ {} ▼]", self.state.style))
                .style(style_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            let bpm = Paragraph::new(format!("BPM: [{}]", self.state.bpm))
                .style(bpm_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            let length = Paragraph::new(format!("Length: [{} ▼]", self.state.length))
                .style(length_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);

            f.render_widget(scale, params_layout_top[0]);
            f.render_widget(style_param, params_layout_top[3]);
            f.render_widget(bpm, params_layout_bottom[0]);
            f.render_widget(length, params_layout_bottom[3]);

            let seed_style = if self.current_focus == InputId::Seed {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            // Format the seed display
            let seed_display = if self.state.seed.is_empty() {
                "Seed (optional): [______]".to_string()
            } else {
                format!("Seed (optional): [{}]", self.state.seed)
            };

            let seed = Paragraph::new(seed_display)
                .style(seed_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(seed, create_track_layout[5]);

            // Generate button
            let generate_style = if self.current_focus == InputId::Generate
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let generate = Paragraph::new("[♫ Generate]")
                .style(generate_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(generate, create_track_layout[7]);

            // Create and render the Load Track panel
            let load_track_block = Block::default()
                .title("Load Track by ID")
                .add_modifier(Modifier::BOLD)
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
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let load_style = if self.current_focus == InputId::Load
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            // Format track ID display
            let track_id_display = if self.state.track_id.is_empty() {
                "Track ID: [________________________________]".to_string()
            } else {
                format!("Track ID: [{}]", self.state.track_id)
            };

            // Render Track ID and Load button
            let track_id = Paragraph::new(track_id_display)
                .style(track_id_style)
                .alignment(Alignment::Center);
            let load = Paragraph::new("[↓ Load]")
                .style(load_style)
                .alignment(Alignment::Center);

            f.render_widget(track_id, track_id_layout[0]);
            f.render_widget(load, track_id_layout[1]);

            // Draw popup if one is active
            if self.state.input_mode == InputMode::ScalePopup
                || self.state.input_mode == InputMode::StylePopup
                || self.state.input_mode == InputMode::LengthPopup
            {
                // Calculate the popup dimensions and position
                let popup_width = 25;
                let popup_height = 15;
                let popup_x = (terminal_width - popup_width) / 2;
                let popup_y = (terminal_height - popup_height) / 2;

                let popup_area = Rect {
                    x: popup_x,
                    y: popup_y,
                    width: popup_width,
                    height: popup_height,
                };

                // Clear the area under the popup
                f.render_widget(Clear, popup_area);

                // Create a block for the popup with appropriate title
                let title = match self.state.input_mode {
                    InputMode::ScalePopup => "Select Scale",
                    InputMode::StylePopup => "Select Style",
                    InputMode::LengthPopup => "Select Length",
                    _ => "",
                };

                let popup_block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::DarkGray));

                f.render_widget(popup_block.clone(), popup_area);

                // Create the inner area for the list
                let inner_popup_area = popup_block.inner(popup_area);

                // Get the appropriate items based on the active popup
                let items: Vec<ListItem> = match self.state.input_mode {
                    InputMode::ScalePopup => self
                        .state
                        .scales
                        .iter()
                        .map(|s| ListItem::new(s.clone()))
                        .collect(),
                    InputMode::StylePopup => self
                        .state
                        .styles
                        .iter()
                        .map(|s| ListItem::new(s.clone()))
                        .collect(),
                    InputMode::LengthPopup => self
                        .state
                        .lengths
                        .iter()
                        .map(|s| ListItem::new(s.clone()))
                        .collect(),
                    _ => vec![],
                };

                // Create the list widget
                let list = List::new(items)
                    .block(Block::default())
                    .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

                // Render the list with state
                f.render_stateful_widget(list, inner_popup_area, &mut self.state.popup_list_state);
            }

            // Show cursor when in editing mode
            if self.state.input_mode == InputMode::Editing {
                match self.current_focus {
                    InputId::Bpm => {
                        // Position cursor after the text in the BPM field
                        let x = params_layout_top[2].x + 6 + self.state.bpm.len() as u16;
                        let y = params_layout_top[2].y;
                        f.set_cursor_position(Position { x, y });
                    }
                    InputId::Seed => {
                        // Position cursor after the text in the seed field
                        let x = create_track_layout[4].x + 17 + self.state.seed.len() as u16;
                        let y = create_track_layout[4].y;
                        f.set_cursor_position(Position { x, y });
                    }
                    InputId::TrackID => {
                        // Position cursor after the text in the track ID field
                        let x = track_id_layout[0].x + 11 + self.state.track_id.len() as u16;
                        let y = track_id_layout[0].y;
                        f.set_cursor_position(Position { x, y });
                    }
                    _ => {}
                }
            }
        })?;
        Ok(())
    }

    pub fn toggle_play(&self) -> (f32, Option<Instant>) {
        let currently_playing = IS_PLAYING.load(Ordering::SeqCst);
        let mut elapsed_play_time: f32 = 0.0;
        let mut start_time: Option<Instant> = None;
        // println!("UI: before - IS_PLAYING = {}", currently_playing);

        if currently_playing {
            // Music is playing, so pause it
            if pause_music().is_ok() {
                IS_PLAYING.store(false, Ordering::SeqCst);
                if let Some(start_time) = self.state.start_time {
                    let elapsed = start_time.elapsed().as_secs_f32();
                    elapsed_play_time += elapsed;
                }
                // println!("Music paused");
            }
        } else {
            // Check if a sender exists without holding the lock during resume_music()
            let should_resume = {
                let sender = get_music_sender().lock().unwrap();
                sender.is_some()
            };

            if should_resume {
                if resume_music().is_ok() {
                    IS_PLAYING.store(true, Ordering::SeqCst);
                    start_time = Some(Instant::now()); // Resume timing
                                                       // println!("Music resumed");
                }
            } else {
                start_music_in_thread(self.state.clone()).unwrap();
                IS_PLAYING.store(true, Ordering::SeqCst);
                elapsed_play_time = 0.0; // Reset time tracking for a new song
                start_time = Some(Instant::now());
                // println!("Music started");
            }
        }
        // println!(
        //     "UI: after - IS_PLAYING = {}",
        //     IS_PLAYING.load(Ordering::SeqCst)
        // );

        (elapsed_play_time, start_time)
    }

    // Method to handle user input
    pub fn handle_input(&mut self) -> std::io::Result<bool> {
        match event::read()? {
            // Handle key events
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                match (self.state.input_mode, code) {
                    // In Scale Popup mode
                    (InputMode::ScalePopup, KeyCode::Esc) => {
                        // Exit popup mode
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::ScalePopup, KeyCode::Enter) => {
                        // Select the current scale and exit popup mode
                        if let Some(selected) = self.state.popup_list_state.selected() {
                            self.state.scale = self.state.scales[selected].clone();
                        }
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::ScalePopup, KeyCode::Up)
                    | (InputMode::ScalePopup, KeyCode::Char('k')) => {
                        // Navigate up in the scale list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected > 0 {
                            selected - 1
                        } else {
                            self.state.scales.len() - 1
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }
                    (InputMode::ScalePopup, KeyCode::Down)
                    | (InputMode::ScalePopup, KeyCode::Char('j')) => {
                        // Navigate down in the scale list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected < self.state.scales.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }

                    // In Style Popup mode
                    (InputMode::StylePopup, KeyCode::Esc) => {
                        // Exit popup mode
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::StylePopup, KeyCode::Enter) => {
                        // Select the current style and exit popup mode
                        if let Some(selected) = self.state.popup_list_state.selected() {
                            self.state.style = self.state.styles[selected].clone();
                        }
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::StylePopup, KeyCode::Up)
                    | (InputMode::StylePopup, KeyCode::Char('k')) => {
                        // Navigate up in the style list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected > 0 {
                            selected - 1
                        } else {
                            self.state.styles.len() - 1
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }
                    (InputMode::StylePopup, KeyCode::Down)
                    | (InputMode::StylePopup, KeyCode::Char('j')) => {
                        // Navigate down in the style list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected < self.state.styles.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }

                    // In Length Popup mode
                    (InputMode::LengthPopup, KeyCode::Esc) => {
                        // Exit popup mode
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::LengthPopup, KeyCode::Enter) => {
                        // Select the current length and exit popup mode
                        if let Some(selected) = self.state.popup_list_state.selected() {
                            self.state.length = self.state.lengths[selected].clone();
                        }
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::LengthPopup, KeyCode::Up)
                    | (InputMode::LengthPopup, KeyCode::Char('k')) => {
                        // Navigate up in the length list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected > 0 {
                            selected - 1
                        } else {
                            self.state.lengths.len() - 1
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }
                    (InputMode::LengthPopup, KeyCode::Down)
                    | (InputMode::LengthPopup, KeyCode::Char('j')) => {
                        // Navigate down in the length list
                        let selected = self.state.popup_list_state.selected().unwrap_or(0);
                        let new_selection = if selected < self.state.lengths.len() - 1 {
                            selected + 1
                        } else {
                            0
                        };
                        self.state.popup_list_state.select(Some(new_selection));
                    }
                    // In Editing mode
                    (InputMode::Editing, KeyCode::Esc) => {
                        // Exit editing mode
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::Editing, KeyCode::Enter) => {
                        // Confirm editing and exit editing mode
                        self.state.input_mode = InputMode::Navigation;
                    }
                    (InputMode::Editing, KeyCode::Backspace) => {
                        // Handle backspace for text fields
                        match self.current_focus {
                            InputId::Bpm => {
                                self.state.bpm.pop();
                            }
                            InputId::Length => {
                                self.state.length.pop();
                            }
                            InputId::Seed => {
                                self.state.seed.pop();
                            }
                            InputId::TrackID => {
                                self.state.track_id.pop();
                            }
                            _ => {}
                        }
                    }
                    (InputMode::Editing, KeyCode::Char(c)) => {
                        // Handle character input for text fields
                        match self.current_focus {
                            InputId::Bpm => {
                                // Only allow numbers for BPM
                                if c.is_numeric() {
                                    self.state.bpm.push(c);
                                }
                            }
                            InputId::Length => {
                                // Allow numbers, spaces, and some text for length (like "30 sec")
                                if c.is_numeric() || c.is_alphabetic() || c == ' ' {
                                    self.state.length.push(c);
                                }
                            }
                            InputId::Seed => {
                                // Allow alphanumeric characters for seed
                                if c.is_alphanumeric() {
                                    self.state.seed.push(c);
                                }
                            }
                            InputId::TrackID => {
                                // Allow alphanumeric and some special characters for track ID
                                if c.is_alphanumeric() || c == '-' || c == '_' {
                                    self.state.track_id.push(c);
                                }
                            }
                            _ => {}
                        }
                    }

                    // In Navigation mode
                    (InputMode::Navigation, KeyCode::Char('q'))
                    | (InputMode::Navigation, KeyCode::Esc) => {
                        // Quit the application with q or Esc
                        return Ok(false);
                    }
                    (InputMode::Navigation, KeyCode::Enter) => {
                        // Handle Enter key in navigation mode
                        match self.current_focus {
                            InputId::Scale => {
                                // Open scale popup
                                self.state.input_mode = InputMode::ScalePopup;
                                // Find the index of the current scale to select it in the popup
                                if let Some(index) = self
                                    .state
                                    .scales
                                    .iter()
                                    .position(|s| s == &self.state.scale)
                                {
                                    self.state.popup_list_state.select(Some(index));
                                }
                            }
                            InputId::Style => {
                                // Open style popup
                                self.state.input_mode = InputMode::StylePopup;
                                // Find the index of the current style to select it in the popup
                                if let Some(index) = self
                                    .state
                                    .styles
                                    .iter()
                                    .position(|s| s == &self.state.style)
                                {
                                    self.state.popup_list_state.select(Some(index));
                                }
                            }
                            InputId::Length => {
                                // Open length popup
                                self.state.input_mode = InputMode::LengthPopup;
                                // Find the index of the current length to select it in the popup
                                if let Some(index) = self
                                    .state
                                    .lengths
                                    .iter()
                                    .position(|s| s == &self.state.length)
                                {
                                    self.state.popup_list_state.select(Some(index));
                                }
                            }
                            InputId::Bpm | InputId::Seed | InputId::TrackID => {
                                // Enter editing mode for these fields
                                self.state.input_mode = InputMode::Editing;
                            }
                            InputId::Generate => {
                                // Handle generate button action
                                // For now, just print a message (would be implemented with actual generation)
                                // In a real app, this would trigger music generation
                            }
                            InputId::Load => {
                                // Handle load button action
                                // For now, just print a message (would be implemented with actual loading)
                                // In a real app, this would load a track by ID
                            }
                            InputId::Rewind => {
                                // Rewind: reset the current song to the beginning.
                            }
                            InputId::PlayPause => {
                                // Handle play/pause toggle
                                (self.state.elapsed_play_time, self.state.start_time) =
                                    self::Tui::<B>::toggle_play(self);
                            }
                            InputId::Skip => {
                                // Fast forward (Skip to the next song)
                            }
                            InputId::Loop => {
                                // Handle loop toggle action
                            }
                        }
                    }
                    (InputMode::Navigation, KeyCode::Up | KeyCode::Char('k')) => {
                        // Navigate up
                        self.current_focus = next_focus(self.current_focus, Direction::Up);
                    }
                    (InputMode::Navigation, KeyCode::Down | KeyCode::Char('j')) => {
                        // Navigate down
                        self.current_focus = next_focus(self.current_focus, Direction::Down);
                    }
                    (InputMode::Navigation, KeyCode::Left | KeyCode::Char('h')) => {
                        // Navigate left
                        self.current_focus = next_focus(self.current_focus, Direction::Left);
                    }
                    (InputMode::Navigation, KeyCode::Right | KeyCode::Char('l')) => {
                        // Navigate right
                        self.current_focus = next_focus(self.current_focus, Direction::Right);
                    }
                    // Handle backward tabbing with Shift+Tab
                    (InputMode::Navigation, KeyCode::BackTab) => {
                        let tab_order = [
                            InputId::Rewind,
                            InputId::PlayPause,
                            InputId::Skip,
                            InputId::Loop,
                            InputId::Scale,
                            InputId::Style,
                            InputId::Bpm,
                            InputId::Length,
                            InputId::Seed,
                            InputId::Generate,
                            InputId::TrackID,
                            InputId::Load,
                        ];
                        if let Some(current_pos) =
                            tab_order.iter().position(|&id| id == self.current_focus)
                        {
                            let prev_pos = if current_pos == 0 {
                                tab_order.len() - 1
                            } else {
                                current_pos - 1
                            };
                            self.current_focus = tab_order[prev_pos];
                        } else {
                            self.current_focus = tab_order[tab_order.len() - 1];
                        }
                    }
                    (InputMode::Navigation, KeyCode::Tab) => {
                        let tab_order = [
                            InputId::Rewind,
                            InputId::PlayPause,
                            InputId::Skip,
                            InputId::Loop,
                            InputId::Scale,
                            InputId::Style,
                            InputId::Bpm,
                            InputId::Length,
                            InputId::Seed,
                            InputId::Generate,
                            InputId::TrackID,
                            InputId::Load,
                        ];
                        if let Some(current_pos) =
                            tab_order.iter().position(|&id| id == self.current_focus)
                        {
                            let next_pos = (current_pos + 1) % tab_order.len();
                            self.current_focus = tab_order[next_pos];
                        } else {
                            self.current_focus = tab_order[0];
                        }
                    }
                    _ => {}
                }
                if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
                    return Ok(false);
                }
            }
            _ => {}
        }
        Ok(true)
    }
}
