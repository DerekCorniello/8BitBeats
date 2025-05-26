// use crate::gen::{get_music_sender, pause_music, resume_music, start_music_in_thread}; // Removed these unused imports
use crossterm::{
    event::{self, Event, KeyCode}, // Removed KeyEvent, KeyModifiers
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph}, // Added List, ListItem
    Terminal,
};

use std::{collections::HashMap, io, sync::OnceLock};

// Removed: use std::time::Instant;

// Define UserAction enum
pub enum UserAction {
    Quit,
    TogglePlayback,
    UpdateInput,
    Navigate,
    SwitchToEditing,
    SwitchToNavigation,
    OpenPopup,
    CyclePopupOption,
    SelectPopupItem,
    GenerateMusic,
    NoOp,
    ToggleLoop,      // Added for the loop button
    AttemptLoadSong, // Renamed from LoadAndPlaySong
    CloseSongIdErrorPopup, // To close the error popup
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum InputId {
    Rewind,
    PlayPause,
    Skip,
    Loop, // Replaced SaveSong
    Scale,
    Style,
    Bpm,
    Length,
    Seed,
    Generate,
    SongLoader,
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
                    (Direction::Left, InputId::Bpm),
                    (Direction::Right, InputId::Length),
                ]),
            },
        );

        graph.insert(
            InputId::Generate,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Seed),
                    (Direction::Down, InputId::SongLoader),
                    (Direction::Left, InputId::Generate),
                    (Direction::Right, InputId::Generate),
                ]),
            },
        );

        graph.insert(
            InputId::SongLoader,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Generate),
                    (Direction::Down, InputId::SongLoader),
                    (Direction::Left, InputId::SongLoader),
                    (Direction::Right, InputId::SongLoader),
                ]),
            },
        );

        graph
    })
}

// fn create_progress_bar... removed as it was unused and referred to AppState.progress

// Input mode to determine how to handle user input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Navigation,
    Editing,
    ScalePopup,
    StylePopup,
    LengthPopup,
    SongLoaderEditing,
    SongIdErrorPopup, // For the error message popup
}

// AppState to store application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub scale: String,
    pub style: String,
    pub bpm: String,
    pub length: String,
    pub seed: String,
    pub input_mode: InputMode,
    pub popup_list_state: ListState,
    pub scales: Vec<String>,
    pub styles: Vec<String>,
    pub lengths: Vec<String>,
    pub is_playing: bool,
    pub current_song_progress: f32,
    pub current_song_elapsed_secs: f32,
    pub current_song_duration_secs: f32,
    pub is_loop_enabled: bool,         // Added for loop state
    pub song_loader_input: String,     // Added for the new song loader input
    pub song_id_error: Option<String>, // For song ID validation errors
    pub current_song_id_display: Option<String>, // To display the current song's ID
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scale: "C".to_string(),
            style: "Pop".to_string(),
            bpm: "120".to_string(),
            length: "5 min".to_string(),
            seed: "".to_string(),
            input_mode: InputMode::Navigation,
            popup_list_state: ListState::default(),
            scales: vec![
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            styles: vec![
                "Pop",
                "Rock",
                "Jazz",
                "Blues",
                "Electronic",
                "Ambient",
                "Classical",
                "Folk",
                "Metal",
                "Reggae",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            lengths: vec!["1 min", "2 min", "3 min", "5 min", "10 min"]
                .into_iter()
                .map(String::from)
                .collect(),
            is_playing: false,
            current_song_progress: 0.0,
            current_song_elapsed_secs: 0.0,
            current_song_duration_secs: 0.0,
            is_loop_enabled: false,           // Default loop state
            song_loader_input: String::new(), // Initialize the new field
            song_id_error: None,              // Initialize error as None
            current_song_id_display: None,     // Initialize as None
        }
    }
}

// TUI struct represents the terminal user interface, parameterized by the type of backend (B)
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    current_focus: InputId,
    state: AppState,
    editing_original_value: Option<String>, // To store value before editing starts
}

// Define a sample rate const, assuming it's the same as in gen.rs
const TUI_SAMPLE_RATE: f32 = 44100.0;

// Helper function to format duration from seconds to MM:SS string
fn format_duration(total_seconds: f32) -> String {
    let minutes = (total_seconds / 60.0).floor() as u32;
    let seconds = (total_seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", minutes, seconds)
}

impl<B: Backend> Tui<B> {
    // Constructor method to create a new Tui instance with the provided backend
    pub fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        Ok(Self {
            terminal,
            current_focus: InputId::PlayPause,
            state: AppState::default(),
            editing_original_value: None, // Initialize as None
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

    // Method to update progress
    pub fn update_progress(&mut self, current_samples: u64, total_samples: u64) {
        if total_samples == 0 {
            self.state.current_song_progress = 0.0;
            self.state.current_song_elapsed_secs = 0.0;
            self.state.current_song_duration_secs = 0.0;
        } else {
            self.state.current_song_progress = (current_samples as f32 / total_samples as f32).min(1.0).max(0.0);
            self.state.current_song_elapsed_secs = current_samples as f32 / TUI_SAMPLE_RATE;
            self.state.current_song_duration_secs = total_samples as f32 / TUI_SAMPLE_RATE;
        }
    }

    // Method to set the string for displaying the current song ID
    pub fn set_current_song_id_display(&mut self, id_display: Option<String>) {
        self.state.current_song_id_display = id_display;
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

            // Calculate the total required height for the app
            let title_height = 8; // Title section height
            let content_height = 24; // Adjusted: Now Playing (8) + Gap (1) + Create New Track (9) + Gap (1) + Load Song (5)
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
                    Constraint::Length(1), // Gap for new panel
                    Constraint::Length(5), // Load Song panel (increased from 4 to 5)
                    Constraint::Min(1),    // Remaining space
                ])
                .split(centered_content_area);

            let now_playing_area = panel_layout[0];
            let create_track_area = panel_layout[2];
            let song_loader_area = panel_layout[4]; // Added area for song loader

            let now_playing_block = Block::default().title("Now Playing").borders(Borders::ALL);
            let inner_now_playing = now_playing_block.inner(now_playing_area);
            f.render_widget(now_playing_block, now_playing_area);

            // Now playing content layout - added seed text row
            let now_playing_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(1), // Song ID text
                    Constraint::Length(1), // Progress Bar
                    Constraint::Length(1), // Progress Text (MM:SS / MM:SS)
                    Constraint::Length(1), // Empty space
                    Constraint::Min(1),    // Controls (takes remaining space)
                ])
                .margin(1)
                .split(inner_now_playing);

            // Seed display text
            let song_id_display_text = format!("Song ID: {}", self.state.current_song_id_display.as_deref().unwrap_or("N/A"));
            let song_id_paragraph = Paragraph::new(song_id_display_text)
                .alignment(Alignment::Center);
            f.render_widget(song_id_paragraph, now_playing_layout[0]);

            // Progress Bar - index adjusted to [1]
            let progress_percentage = (self.state.current_song_progress * 100.0) as u16;
            let progress_bar = Gauge::default()
                .block(Block::default())
                .gauge_style(Style::default().fg(Color::Blue).bg(Color::DarkGray))
                .percent(progress_percentage)
                .label(format!("{}%", progress_percentage));
            f.render_widget(progress_bar, now_playing_layout[1]);

            // Progress Text (MM:SS / MM:SS) - index adjusted to [2]
            let elapsed_str = format_duration(self.state.current_song_elapsed_secs);
            let total_str = format_duration(self.state.current_song_duration_secs);
            let progress_text = Paragraph::new(format!("{} / {}", elapsed_str, total_str))
                .alignment(Alignment::Center);
            f.render_widget(progress_text, now_playing_layout[2]);

            // Controls layout for Now Playing section - index adjusted to [4]
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

            // Dynamically set Play/Pause button text
            let play_pause_text = if self.state.is_playing {
                "[|| Pause]" // Pause symbol
            } else {
                "  [▷ Play]" // Play symbol
            };
            let play_pause = Paragraph::new(play_pause_text)
                .style(play_pause_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            let skip = Paragraph::new("[>> Skip]")
                .style(skip_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            let loop_button_text = if self.state.is_loop_enabled {
                "[↻ Disable Loop]"
            } else {
                "[↻ Enable Loop]"
            };
            let loop_widget = Paragraph::new(loop_button_text)
                .style(loop_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            f.render_widget(rewind, control_layout[0]);
            f.render_widget(play_pause, control_layout[1]);
            f.render_widget(skip, control_layout[2]);
            f.render_widget(loop_widget, control_layout[3]);

            // Create and render the Create New Track panel
            let create_track_block = Block::default()
                .title("Create New Track")
                .borders(Borders::ALL);

            let inner_create_track = create_track_block.inner(create_track_area);
            f.render_widget(create_track_block, create_track_area);

            // --- START TARGETED REINSTATEMENT FOR "SCALE" WIDGET ONLY ---
            // Create the main vertical layout for the panel's content
            let create_track_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(1), // Parameters row 1 (Scale, Style)
                    Constraint::Length(1), // Space
                    Constraint::Length(1), // Parameters row 2 (BPM, Length)
                    Constraint::Length(1), // Space
                    Constraint::Length(1), // Seed row
                    Constraint::Length(1), // Space
                    Constraint::Length(1), // Generate button
                ])
                .split(inner_create_track);

            // Create the horizontal layout for the first row of parameters
            let params_layout_top = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4), // Cell for Scale
                    Constraint::Ratio(1, 4), // Empty cell
                    Constraint::Ratio(1, 4), // Empty cell
                    Constraint::Ratio(1, 4), // Cell for Style (will be empty for now)
                ])
                .split(create_track_layout[0]); // Split the first row

            // Define style for the Scale widget (needed for focus indication)
            let scale_style = if self.current_focus == InputId::Scale {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            // Create the Scale widget Paragraph
            let scale_widget_paragraph =
                Paragraph::new(format!("Scale: [ {} ▼]", self.state.scale))
                    .style(scale_style)
                    .add_modifier(Modifier::BOLD)
                    .alignment(Alignment::Center);

            // Render ONLY the Scale widget into its cell
            f.render_widget(scale_widget_paragraph, params_layout_top[0]);

            // --- UNCOMMENT THE REST OF THE CREATE NEW TRACK PANEL CONTENT ---
            // (The following was part of the full reinstatement, now uncommented)
            let params_layout_bottom = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                ])
                .split(create_track_layout[2]);

            // Style definitions for style_style, bpm_style, length_style (scale_style already defined)
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

            // Paragraph definitions for style_param, bpm, length (scale_widget_paragraph already defined as scale)
            // Renaming scale_widget_paragraph to scale for consistency with original code if needed, or use scale_widget_paragraph directly.
            // Assuming `scale` was the variable name for the scale paragraph before simplification. Let's use `scale_widget_paragraph` for clarity here.

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

            // f.render_widget(scale_widget_paragraph, params_layout_top[0]); // Already rendered
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

            let seed_display_string = if self.state.seed.is_empty() {
                "Seed (optional): []".to_string()
            } else {
                format!("Seed (optional): [{}]", self.state.seed)
            };

            let seed = Paragraph::new(seed_display_string.clone())
                .style(seed_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(seed, create_track_layout[4]);

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
            f.render_widget(generate, create_track_layout[6]);
            // --- END UNCOMMENT AND RESTORE ---

            // Define song_loader_block and inner_song_loader_area early for cursor logic
            let song_loader_block = Block::default()
                .title("Load Song (Enter to Load)")
                .borders(Borders::ALL);
            // This inner area is what the cursor logic will use for its coordinate calculations
            let inner_song_loader_area_for_cursor_and_render =
                song_loader_block.inner(song_loader_area);

            // Show cursor when in editing mode - Ensure this uses correct layout variables
            if self.state.input_mode == InputMode::Editing
                || self.state.input_mode == InputMode::SongLoaderEditing
            {
                match self.current_focus {
                    InputId::Bpm => {
                        let bpm_widget_cell_area = params_layout_bottom[0]; // Cell for BPM
                        let row_y = create_track_layout[2].y; // Y of the row containing BPM
                        let full_text_content = format!("BPM: [{}]", self.state.bpm);
                        let text_prefix_len = "BPM: [".len() as u16;

                        let centered_text_start_x = bpm_widget_cell_area.x
                            + (bpm_widget_cell_area.width / 2)
                                .saturating_sub(full_text_content.len() as u16 / 2);

                        let x =
                            centered_text_start_x + text_prefix_len + self.state.bpm.len() as u16;
                        let y = row_y;
                        f.set_cursor(x, y);
                    }
                    InputId::Seed => {
                        let seed_widget_row_area = create_track_layout[4]; // Row for Seed
                        let text_prefix_len = "Seed (optional): [".len() as u16;
                        // seed_display_string is defined above in the rendering part
                        let centered_text_start_x = seed_widget_row_area.x
                            + (seed_widget_row_area.width / 2)
                                .saturating_sub(seed_display_string.len() as u16 / 2);
                        let x =
                            centered_text_start_x + text_prefix_len + self.state.seed.len() as u16;
                        let y = seed_widget_row_area.y;
                        f.set_cursor(x, y);
                    }
                    InputId::SongLoader => {
                        // Added cursor handling for SongLoader
                        let song_loader_text_prefix = "Load: [";
                        let current_input_value = &self.state.song_loader_input;
                        // Text used for measuring total width for centering
                        let text_for_width_measurement =
                            format!("{}{}]", song_loader_text_prefix, current_input_value);

                        // Calculate starting X position for centered text
                        let centered_text_start_x = inner_song_loader_area_for_cursor_and_render.x
                            + (inner_song_loader_area_for_cursor_and_render.width / 2)
                                .saturating_sub(text_for_width_measurement.len() as u16 / 2);

                        let cursor_x = centered_text_start_x
                            + song_loader_text_prefix.len() as u16
                            + current_input_value.len() as u16;

                        // Vertically center the cursor line within inner_song_loader_area (inner height should be 3 if parent is 5)
                        let cursor_y = inner_song_loader_area_for_cursor_and_render.y
                            + inner_song_loader_area_for_cursor_and_render.height / 2;

                        f.set_cursor(cursor_x, cursor_y);
                    }
                    _ => {}
                }
            }

            // Render Load Song panel
            // song_loader_block is already defined, inner_song_loader_area_for_cursor_and_render holds the inner rect
            f.render_widget(song_loader_block, song_loader_area); // Render the block itself

            let song_loader_input_style = if self.current_focus == InputId::SongLoader {
                if self.state.input_mode == InputMode::SongLoaderEditing {
                    Style::default().fg(Color::Green) // Editing SongLoader
                } else {
                    Style::default().fg(Color::Yellow) // Navigating to SongLoader
                }
            } else {
                Style::default()
            };

            let song_loader_display_text = if self.state.input_mode == InputMode::SongLoaderEditing
            {
                format!("Load: [{}]", self.state.song_loader_input)
            } else if self.state.song_loader_input.is_empty() {
                "Load: []".to_string()
            } else {
                format!("Load: [{}]", self.state.song_loader_input)
            };

            let song_loader_paragraph = Paragraph::new(song_loader_display_text)
                .style(song_loader_input_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);

            // Render the paragraph centered within the inner_song_loader_area
            // Create a layout for vertical centering within the 3 lines of inner_song_loader_area
            let centered_loader_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Ratio(1, 3), // Top padding
                    Constraint::Length(1),   // Text line
                    Constraint::Ratio(1, 3), // Bottom padding
                ])
                .split(inner_song_loader_area_for_cursor_and_render); // Use the hoisted Rect

            f.render_widget(song_loader_paragraph, centered_loader_layout[1]);

            // Popup rendering section (ensure it is present if popups are used)
            if self.state.input_mode == InputMode::ScalePopup
                || self.state.input_mode == InputMode::StylePopup
                || self.state.input_mode == InputMode::LengthPopup
            {
                let popup_width = 25;
                let popup_height = 15;
                let popup_x = (f.size().width - popup_width) / 2;
                let popup_y = (f.size().height - popup_height) / 2;

                let popup_area = Rect {
                    x: popup_x,
                    y: popup_y,
                    width: popup_width,
                    height: popup_height,
                };

                f.render_widget(Clear, popup_area);

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
                let inner_popup_area = popup_block.inner(popup_area);

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
                let list_widget = List::new(items)
                    .block(Block::default())
                    .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));
                f.render_stateful_widget(
                    list_widget,
                    inner_popup_area,
                    &mut self.state.popup_list_state,
                );
            }

            // Song ID Error Popup
            if self.state.input_mode == InputMode::SongIdErrorPopup {
                if let Some(error_msg) = &self.state.song_id_error {
                    let popup_width = 60; // Wider for potentially longer error messages
                    let lines = textwrap::wrap(error_msg, popup_width as usize - 4); // -4 for padding/borders
                    let popup_height = (lines.len() + 4) as u16; // +2 for title/instruction, +2 for borders

                    let popup_x = (f.size().width.saturating_sub(popup_width)) / 2;
                    let popup_y = (f.size().height.saturating_sub(popup_height)) / 2;

                    let popup_area = Rect {
                        x: popup_x,
                        y: popup_y,
                        width: popup_width,
                        height: popup_height,
                    };

                    f.render_widget(Clear, popup_area); // Clear the area for the popup

                    let popup_block = Block::default()
                        .title("Invalid Song ID")
                        .borders(Borders::ALL)
                        .style(Style::default().bg(Color::DarkGray).fg(Color::Red)); // Red text for error

                    let inner_popup_area = popup_block.inner(popup_area);
                    f.render_widget(popup_block.clone(), popup_area);

                    // Layout for error message and instruction
                    let popup_content_layout = Layout::default()
                        .direction(LayoutDirection::Vertical)
                        .margin(1) // Margin within the inner area
                        .constraints([
                            Constraint::Min(lines.len() as u16), // For the error message lines
                            Constraint::Length(1),               // For the instruction
                        ])
                        .split(inner_popup_area);

                    let error_paragraph = Paragraph::new(error_msg.clone())
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .style(Style::default().fg(Color::White)); // White text on dark gray bg
                    f.render_widget(error_paragraph, popup_content_layout[0]);

                    let instruction_paragraph = Paragraph::new("Press Enter or Esc to correct.")
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(Color::Yellow));
                    f.render_widget(instruction_paragraph, popup_content_layout[1]);
                }
            }
        })?;
        Ok(())
    }

    pub fn get_current_app_state(&self) -> AppState {
        self.state.clone()
    }

    // Method to get the current focused InputId
    // pub fn current_focus(&self) -> InputId { // Removed as unused
    //     self.current_focus
    // }

    // Method to explicitly set the playing state, e.g., after music generation
    pub fn set_playing_state(&mut self, is_playing: bool) {
        self.state.is_playing = is_playing;
    }

    pub fn is_paused(&self) -> bool {
        !self.state.is_playing
    }

    // Method to clear the song loader input field
    pub fn clear_song_loader_input(&mut self) {
        self.state.song_loader_input.clear();
    }

    // Method to set focus to the PlayPause button
    pub fn focus_on_play_pause(&mut self) {
        self.current_focus = InputId::PlayPause;
        self.state.input_mode = InputMode::Navigation; // Ensure navigation mode
    }

    // Method to show a song ID error
    pub fn show_song_id_error(&mut self, error_message: String) {
        self.state.song_id_error = Some(error_message);
        self.state.input_mode = InputMode::SongIdErrorPopup;
    }

    // Method to handle user input
    pub fn handle_input(&mut self) -> std::io::Result<UserAction> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // self.state.status_message = None; // Optional: Clear previous status message on new input

                if key.code == KeyCode::Char('q') && self.state.input_mode == InputMode::Navigation
                {
                    return Ok(UserAction::Quit);
                }

                match self.state.input_mode {
                    InputMode::Navigation => {
                        match key.code {
                            KeyCode::Up => {
                                self.current_focus = next_focus(self.current_focus, Direction::Up);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Down => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Down);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Left => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Left);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Right => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Right);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Enter => match self.current_focus {
                                InputId::PlayPause => {
                                    /* self.state.is_playing = !self.state.is_playing; */
                                    Ok(UserAction::TogglePlayback)
                                }
                                InputId::Loop => {
                                    self.state.is_loop_enabled = !self.state.is_loop_enabled;
                                    Ok(UserAction::ToggleLoop)
                                }
                                InputId::Scale => {
                                    self.state.input_mode = InputMode::ScalePopup;
                                    self.state.popup_list_state.select(Some(0));
                                    Ok(UserAction::OpenPopup)
                                }
                                InputId::Style => {
                                    self.state.input_mode = InputMode::StylePopup;
                                    self.state.popup_list_state.select(Some(0));
                                    Ok(UserAction::OpenPopup)
                                }
                                InputId::Length => {
                                    self.state.input_mode = InputMode::LengthPopup;
                                    self.state.popup_list_state.select(Some(0));
                                    Ok(UserAction::OpenPopup)
                                }
                                InputId::Bpm => {
                                    self.editing_original_value = Some(self.state.bpm.clone());
                                    self.state.input_mode = InputMode::Editing;
                                    Ok(UserAction::SwitchToEditing)
                                }
                                InputId::Seed => {
                                    self.editing_original_value = Some(self.state.seed.clone());
                                    self.state.input_mode = InputMode::Editing;
                                    Ok(UserAction::SwitchToEditing)
                                }
                                InputId::Generate => Ok(UserAction::GenerateMusic),
                                InputId::SongLoader => {
                                    // Added SongLoader Enter in Navigation mode
                                    self.editing_original_value =
                                        Some(self.state.song_loader_input.clone());
                                    self.state.input_mode = InputMode::SongLoaderEditing;
                                    Ok(UserAction::SwitchToEditing)
                                }
                                _ => Ok(UserAction::NoOp),
                            },
                            _ => Ok(UserAction::NoOp),
                        }
                    }
                    InputMode::Editing => {
                        match self.current_focus {
                            InputId::Bpm => match key.code {
                                KeyCode::Enter => {
                                    self.editing_original_value = None;
                                    self.state.input_mode = InputMode::Navigation;
                                    Ok(UserAction::SwitchToNavigation)
                                }
                                KeyCode::Esc => {
                                    if let Some(val) = self.editing_original_value.take() {
                                        self.state.bpm = val;
                                    }
                                    self.state.input_mode = InputMode::Navigation;
                                    Ok(UserAction::SwitchToNavigation)
                                }
                                KeyCode::Char(c) => {
                                    if c.is_ascii_digit() && self.state.bpm.len() < 3 {
                                        self.state.bpm.push(c);
                                        Ok(UserAction::UpdateInput)
                                    } else {
                                        Ok(UserAction::NoOp)
                                    }
                                }
                                KeyCode::Backspace => {
                                    self.state.bpm.pop();
                                    Ok(UserAction::UpdateInput)
                                }
                                _ => Ok(UserAction::NoOp),
                            },
                            InputId::Seed => match key.code {
                                KeyCode::Enter => {
                                    self.editing_original_value = None;
                                    self.state.input_mode = InputMode::Navigation;
                                    Ok(UserAction::SwitchToNavigation)
                                }
                                KeyCode::Esc => {
                                    if let Some(val) = self.editing_original_value.take() {
                                        self.state.seed = val;
                                    }
                                    self.state.input_mode = InputMode::Navigation;
                                    Ok(UserAction::SwitchToNavigation)
                                }
                                KeyCode::Char(c) => {
                                    if c.is_ascii_digit() {
                                        self.state.seed.push(c);
                                        Ok(UserAction::UpdateInput)
                                    } else {
                                        Ok(UserAction::NoOp)
                                    }
                                }
                                KeyCode::Backspace => {
                                    self.state.seed.pop();
                                    Ok(UserAction::UpdateInput)
                                }
                                _ => Ok(UserAction::NoOp),
                            },
                            _ => Ok(UserAction::NoOp), // Should not happen if current_focus is Bpm, Seed, or QuickLoadString
                        }
                    }
                    InputMode::ScalePopup | InputMode::StylePopup | InputMode::LengthPopup => {
                        match key.code {
                            KeyCode::Esc => {
                                self.state.input_mode = InputMode::Navigation;
                                Ok(UserAction::SwitchToNavigation)
                            }
                            KeyCode::Up => {
                                let list_len = match self.state.input_mode {
                                    InputMode::ScalePopup => self.state.scales.len(),
                                    InputMode::StylePopup => self.state.styles.len(),
                                    InputMode::LengthPopup => self.state.lengths.len(),
                                    _ => 0, // Should not happen
                                };
                                if list_len > 0 {
                                    let current_selection =
                                        self.state.popup_list_state.selected().unwrap_or(0);
                                    let next_selection = if current_selection == 0 {
                                        list_len - 1
                                    } else {
                                        current_selection - 1
                                    };
                                    self.state.popup_list_state.select(Some(next_selection));
                                }
                                Ok(UserAction::CyclePopupOption)
                            }
                            KeyCode::Down => {
                                let list_len = match self.state.input_mode {
                                    InputMode::ScalePopup => self.state.scales.len(),
                                    InputMode::StylePopup => self.state.styles.len(),
                                    InputMode::LengthPopup => self.state.lengths.len(),
                                    _ => 0, // Should not happen
                                };
                                if list_len > 0 {
                                    let current_selection =
                                        self.state.popup_list_state.selected().unwrap_or(0);
                                    let next_selection = (current_selection + 1) % list_len;
                                    self.state.popup_list_state.select(Some(next_selection));
                                }
                                Ok(UserAction::CyclePopupOption)
                            }
                            KeyCode::Enter => {
                                if let Some(selected_index) = self.state.popup_list_state.selected()
                                {
                                    // Determine which popup is active by checking self.current_focus,
                                    // as this was the field that triggered the popup.
                                    match self.current_focus {
                                        InputId::Scale => {
                                            if selected_index < self.state.scales.len() {
                                                self.state.scale =
                                                    self.state.scales[selected_index].clone();
                                            }
                                        }
                                        InputId::Style => {
                                            if selected_index < self.state.styles.len() {
                                                self.state.style =
                                                    self.state.styles[selected_index].clone();
                                            }
                                        }
                                        InputId::Length => {
                                            if selected_index < self.state.lengths.len() {
                                                self.state.length =
                                                    self.state.lengths[selected_index].clone();
                                            }
                                        }
                                        _ => {} // Should not happen, current_focus should be one of the above
                                    }
                                }
                                self.state.input_mode = InputMode::Navigation;
                                Ok(UserAction::SelectPopupItem)
                            }
                            _ => Ok(UserAction::NoOp),
                        }
                    }
                    InputMode::SongLoaderEditing => {
                        // Added new input mode handling
                        match key.code {
                            KeyCode::Enter => {
                                self.editing_original_value = None;
                                self.state.input_mode = InputMode::Navigation;
                                // Potentially trim whitespace or validate before sending
                                Ok(UserAction::AttemptLoadSong)
                            }
                            KeyCode::Esc => {
                                if let Some(val) = self.editing_original_value.take() {
                                    self.state.song_loader_input = val;
                                }
                                self.state.input_mode = InputMode::Navigation;
                                Ok(UserAction::SwitchToNavigation)
                            }
                            KeyCode::Char(c) => {
                                if c.is_alphanumeric() || c == '-' {
                                    self.state.song_loader_input.push(c);
                                    Ok(UserAction::UpdateInput)
                                } else {
                                    Ok(UserAction::NoOp)
                                }
                            }
                            KeyCode::Backspace => {
                                self.state.song_loader_input.pop();
                                Ok(UserAction::UpdateInput)
                            }
                            _ => Ok(UserAction::NoOp),
                        }
                    }
                    InputMode::SongIdErrorPopup => {
                        // Handle input for the error popup
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                self.state.input_mode = InputMode::SongLoaderEditing; // Go back to editing the ID
                                self.state.song_id_error = None; // Clear the error
                                Ok(UserAction::CloseSongIdErrorPopup)
                            }
                            _ => Ok(UserAction::NoOp), // Ignore other keys
                        }
                    }
                }
            } else {
                Ok(UserAction::NoOp) // No key event if event::read() fails or is not a Key event
            }
        } else {
            Ok(UserAction::NoOp) // No event polled within the timeout
        }
    }
}

// Reinstate the next_focus function
fn next_focus(current: InputId, direction: Direction) -> InputId {
    get_input_graph()
        .get(&current)
        .and_then(|node| node.neighbors.get(&direction).copied())
        .unwrap_or(current)
}
