use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph},
    Terminal,
};

use std::{collections::HashMap, io, sync::OnceLock};


/* UserAction - Represents all possible actions a user can trigger in the TUI.
 *
 * This enum is used to communicate user intentions from the input handling logic
 * to the main application loop, which then acts upon these actions.
 */
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
    GenerateRandomMusic,
    NoOp,
    AttemptLoadSong,
    CloseSongIdErrorPopup,
    RewindSong,
    FastForwardSong,
    ToggleHelp,
}

/* Direction - Represents navigational directions within the TUI.
 *
 * Used for navigating between focusable UI elements.
 */
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/* InputId - Uniquely identifies each interactable UI element.
 *
 * This enum is used to track the currently focused UI element and to define
 * the navigation graph between elements.
 */
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum InputId {
    Rewind,
    PlayPause,
    Skip,
    Scale,
    Style,
    Bpm,
    Length,
    Seed,
    Generate,
    GenerateRandom,
    SongLoader,
}

/* InputNode - Represents a node in the TUI navigation graph.
 *
 * Each node corresponds to an `InputId` and stores its neighbors in different
 * navigation directions.
 *
 * fields:
 *     - neighbors (HashMap<Direction, InputId>): Maps navigation `Direction` to the `InputId` of the neighboring element.
 */
#[derive(Debug)]
struct InputNode {
    neighbors: HashMap<Direction, InputId>,
}

// INPUT_GRAPH defines the static navigation map between UI elements.
// It uses InputId as keys and InputNode to define reachable neighbors.
static INPUT_GRAPH: OnceLock<HashMap<InputId, InputNode>> = OnceLock::new();

/* get_input_graph - Retrieves or initializes the TUI navigation graph.
 *
 * This function provides access to the `INPUT_GRAPH`. If the graph has not
 * been initialized yet, this function will build it. The graph defines how
 * focus moves between different UI elements (identified by `InputId`) based
 * on directional input.
 *
 * inputs:
 *     - None
 *
 * outputs:
 *     - &'static HashMap<InputId, InputNode> : A reference to the static navigation graph.
 */
fn get_input_graph() -> &'static HashMap<InputId, InputNode> {
    INPUT_GRAPH.get_or_init(|| {
        let mut graph = HashMap::new();
        graph.insert(
            InputId::Rewind,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Right, InputId::PlayPause),
                    (Direction::Left, InputId::Skip),
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
                    (Direction::Right, InputId::Rewind),
                    (Direction::Left, InputId::PlayPause),
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
                    (Direction::Up, InputId::Skip),
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
                    (Direction::Down, InputId::GenerateRandom),
                    (Direction::Left, InputId::Generate),
                    (Direction::Right, InputId::Generate),
                ]),
            },
        );

        graph.insert(
            InputId::GenerateRandom,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::Generate),
                    (Direction::Down, InputId::SongLoader),
                    (Direction::Left, InputId::GenerateRandom),
                    (Direction::Right, InputId::GenerateRandom),
                ]),
            },
        );

        graph.insert(
            InputId::SongLoader,
            InputNode {
                neighbors: HashMap::from([
                    (Direction::Up, InputId::GenerateRandom),
                    (Direction::Down, InputId::SongLoader),
                    (Direction::Left, InputId::SongLoader),
                    (Direction::Right, InputId::SongLoader),
                ]),
            },
        );

        graph
    })
}

/* InputMode - Defines the current mode of interaction within the TUI.
 *
 * The input mode determines how key presses are interpreted, for example,
 * whether they navigate UI elements, edit text, or interact with a popup.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Navigation,
    Editing,
    ScalePopup,
    StylePopup,
    LengthPopup,
    SongLoaderEditing,
    SongIdErrorPopup,
}

/* AppState - Holds the overall state of the TUI application.
 *
 * This struct centralizes all data that the TUI needs to render itself
 * and respond to user interactions. It includes UI parameters, playback status,
 * progress information, and input field values.
 *
 * fields:
 *     - scale (String): The selected musical scale for generation.
 *     - style (String): The selected musical style for generation.
 *     - bpm (String): The selected beats per minute for generation.
 *     - length (String): The selected length for music generation.
 *     - seed (String): The seed for random number generation, affecting music output.
 *     - input_mode (InputMode): The current input mode of the TUI.
 *     - popup_list_state (ListState): State for managing selection in pop-up lists.
 *     - scales (Vec<String>): List of available musical scales.
 *     - styles (Vec<String>): List of available musical styles.
 *     - lengths (Vec<String>): List of available music lengths.
 *     - is_playing (bool): True if music is currently playing, false otherwise.
 *     - current_song_progress (f32): Playback progress of the current song (0.0 to 1.0).
 *     - current_song_elapsed_secs (f32): Elapsed playback time of the current song in seconds.
 *     - current_song_duration_secs (f32): Total duration of the current song in seconds.
 *     - song_loader_input (String): User input for loading a song by ID.
 *     - song_id_error (Option<String>): Stores an error message if song ID loading fails.
 *     - current_song_id_display (Option<String>): The ID of the currently playing/loaded song.
 *     - show_help (bool): True if the help menu should be displayed.
 */
#[derive(Debug, Clone)]
pub struct AppState {
    pub scale: String,
    pub style: String,
    pub bpm: String,
    pub length: String,
    pub seed: String,
    pub input_mode: InputMode,
    pub popup_list_state: ListState,
    pub is_random: bool,
    pub scales: Vec<String>,
    pub styles: Vec<String>,
    pub lengths: Vec<String>,
    pub is_playing: bool,
    pub current_song_progress: f32,
    pub current_song_elapsed_secs: f32,
    pub current_song_duration_secs: f32,
    pub song_loader_input: String,
    pub song_id_error: Option<String>,
    pub current_song_id_display: Option<String>,
    pub show_help: bool,
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
            is_random: false,
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
            song_loader_input: String::new(),
            song_id_error: None,
            current_song_id_display: None,
            show_help: false,
        }
    }
}

/* Tui - Manages the terminal user interface for the 8BitBeats application.
 *
 * This struct is responsible for initializing and drawing the TUI, handling
 * user input, and managing the application's visual state (`AppState`).
 * It is generic over a `Backend` type, allowing it to work with different
 * terminal backends (e.g., Crossterm).
 *
 * fields:
 *     - terminal (Terminal<B>): The terminal instance used for drawing.
 *     - current_focus (InputId): The UI element that currently has focus.
 *     - state (AppState): The current state of the application's UI.
 *     - editing_original_value (Option<String>): Stores the original value of a field when editing begins.
 */
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    current_focus: InputId,
    state: AppState,
    editing_original_value: Option<String>,
}

// TUI_SAMPLE_RATE: Assumed audio sample rate, used for time calculations in the TUI.
// This should ideally be consistent with the actual sample rate used in `gen.rs`.
const TUI_SAMPLE_RATE: f32 = 44100.0;

/* format_duration - Formats a duration from total seconds into a MM:SS string.
 *
 * This is a helper function used to display time values in a user-friendly format.
 *
 * inputs:
 *     - total_seconds (f32): The total duration in seconds.
 *
 * outputs:
 *     - String : The duration formatted as "MM:SS".
 */
fn format_duration(total_seconds: f32) -> String {
    let minutes = (total_seconds / 60.0).floor() as u32;
    let seconds = (total_seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", minutes, seconds)
}

impl<B: Backend> Tui<B> {
    /* new - Creates a new `Tui` instance.
     *
     * Initializes the terminal and sets up the initial state of the TUI.
     *
     * inputs:
     *     - backend (B): The terminal backend to use.
     *
     * outputs:
     *     - Result<Self, Box<dyn std::error::Error>> : The new `Tui` instance or an error.
     */
    pub fn new(backend: B) -> Result<Self, Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        Ok(Self {
            terminal,
            current_focus: InputId::PlayPause,
            state: AppState::default(),
            editing_original_value: None,
        })
    }

    /* setup - Initializes the terminal for TUI interaction.
     *
     * This method enables raw mode and switches to the alternate screen buffer.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - Result<(), Box<dyn std::error::Error>> : Ok on success, or an error.
     */
    pub fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Ok(())
    }

    /* teardown - Cleans up the terminal after TUI interaction.
     *
     * This method disables raw mode, leaves the alternate screen buffer,
     * and shows the cursor.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - Result<(), Box<dyn std::error::Error>> : Ok on success, or an error.
     */
    pub fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /* update_progress - Updates the song progress information in the TUI state.
     *
     * Calculates and stores the current song progress percentage, elapsed time,
     * and total duration based on sample counts.
     * It handles cases where the song is paused or has just ended/reset.
     *
     * inputs:
     *     - &mut self
     *     - current_samples (u64): The number of samples played so far.
     *     - total_samples (u64): The total number of samples in the song.
     *
     * outputs:
     *     - None
     */
    pub fn update_progress(&mut self, current_samples: u64, total_samples: u64) {
        // When paused, only update total duration for new songs; don't change progress/elapsed time.
        if !self.state.is_playing && total_samples > 0 {
            let new_duration_secs = total_samples as f32 / TUI_SAMPLE_RATE;
            if self.state.current_song_duration_secs != new_duration_secs {
                self.state.current_song_duration_secs = new_duration_secs;
                // If a new song is loaded paused, reset its progress to 0.
                self.state.current_song_progress = 0.0;
                self.state.current_song_elapsed_secs = 0.0;
            }
            return; // Do not update visible progress if paused.
        }

        if total_samples == 0 { // Song ended, reset, or initial state.
            self.state.current_song_progress = 0.0;
            self.state.current_song_elapsed_secs = 0.0;
            self.state.current_song_duration_secs = 0.0;
        } else {
            self.state.current_song_progress = (current_samples as f32 / total_samples as f32).clamp(0.0, 1.0);
            self.state.current_song_elapsed_secs = current_samples as f32 / TUI_SAMPLE_RATE;
            // Ensure duration is updated for the first progress report of a song.
            let new_duration_secs = total_samples as f32 / TUI_SAMPLE_RATE;
            if self.state.current_song_duration_secs == 0.0 || self.state.current_song_duration_secs != new_duration_secs {
                self.state.current_song_duration_secs = new_duration_secs;
            }
        }
    }

    /* set_current_song_id_display - Sets the string for displaying the current song ID.
     *
     * inputs:
     *     - &mut self
     *     - id_display (Option<String>): The song ID string to display, or None to clear it.
     *
     * outputs:
     *     - None
     */
    pub fn set_current_song_id_display(&mut self, id_display: Option<String>) {
        self.state.current_song_id_display = id_display;
    }

    /* draw - Renders the entire TUI to the terminal.
     *
     * This is the main rendering loop. It defines the layout of all UI components,
     * styles them according to the current `AppState` (including focus, input mode,
     * and playback status), and draws them to the terminal using the provided backend.
     * It also handles displaying popups and the help menu when active.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - Result<(), Box<dyn std::error::Error>> : Ok on success, or an error if drawing fails.
     */
    pub fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            static MIN_WIDTH: u16 = 80;
            static MIN_HEIGHT: u16 = 25;

            let size = f.size();
            let terminal_width = size.width;
            let terminal_height = size.height;

            if terminal_width < MIN_WIDTH || terminal_height < MIN_HEIGHT {
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

            let title_height = 8; // Title section height
            let content_height = 24; // Content area: Now Playing (8) + Gap (1) + Create New Track (9) + Gap (1) + Load Song (5)
            let help_hint_height = 1;
            let total_app_content_height = title_height + content_height + help_hint_height;

            let v_padding = (terminal_height.saturating_sub(total_app_content_height)) / 2;

            let app_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(v_padding),      // Top padding
                    Constraint::Length(title_height),   // Title
                    Constraint::Length(content_height), // Content
                    Constraint::Length(help_hint_height),// Footer for help hint
                    Constraint::Min(0),                 // Bottom padding (flexible)
                ])
                .split(size);

            let title_area = app_layout[1];
            let content_area = app_layout[2];
            let footer_area = app_layout[3];

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

            let title_lines: Vec<Line> = ascii_art
                .iter()
                .map(|&line| Line::from(Span::styled(line, Style::default().fg(Color::Blue))))
                .collect();

            let title_paragraph = Paragraph::new(title_lines)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            f.render_widget(title_paragraph, title_area);

            let content_width_percentage = 80; // Use 80% of available width for content
            let mut content_width = (content_area.width as u32 * content_width_percentage / 100) as u16;

            content_width = std::cmp::min(content_width, 100); // Max content width of 100 characters

            let h_padding = (content_area.width.saturating_sub(content_width)) / 2;

            let centered_content_area = Rect {
                x: content_area.x + h_padding,
                y: content_area.y,
                width: content_width,
                height: content_area.height,
            };

            let panel_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(8), // Now Playing panel
                    Constraint::Length(1), // Gap
                    Constraint::Length(11), // Create New Track panel
                    Constraint::Length(1), // Gap
                    Constraint::Length(5), // Load Song panel
                    Constraint::Min(1),    // Remaining space
                ])
                .split(centered_content_area);

            let now_playing_area = panel_layout[0];
            let create_track_area = panel_layout[2];
            let song_loader_area = panel_layout[4];

            let now_playing_block = Block::default().title("Now Playing").borders(Borders::ALL);
            let inner_now_playing = now_playing_block.inner(now_playing_area);
            f.render_widget(now_playing_block, now_playing_area);

            // Layout for elements within the "Now Playing" panel
            let now_playing_layout = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([
                    Constraint::Length(1), // Song ID text
                    Constraint::Length(1), // Progress Bar
                    Constraint::Length(1), // Progress Text (MM:SS / MM:SS)
                    Constraint::Length(1), // Empty space
                    Constraint::Min(1),    // Controls row
                ])
                .margin(1)
                .split(inner_now_playing);

            let song_id_display_text = format!("Song ID: {}", self.state.current_song_id_display.as_deref().unwrap_or("N/A"));
            let song_id_paragraph = Paragraph::new(song_id_display_text)
                .alignment(Alignment::Center);
            f.render_widget(song_id_paragraph, now_playing_layout[0]);

            // Progress Bar
            let progress_percentage = (self.state.current_song_progress * 100.0) as u16;
            let progress_bar = Gauge::default()
                .block(Block::default())
                .gauge_style(Style::default().fg(Color::Blue).bg(Color::DarkGray))
                .percent(progress_percentage)
                .label(format!("{}%", progress_percentage));
            f.render_widget(progress_bar, now_playing_layout[1]);

            // Progress Text (MM:SS / MM:SS)
            let elapsed_str = format_duration(self.state.current_song_elapsed_secs);
            let total_str = format_duration(self.state.current_song_duration_secs);
            let progress_text = Paragraph::new(format!("{} / {}", elapsed_str, total_str))
                .alignment(Alignment::Center);
            f.render_widget(progress_text, now_playing_layout[2]);

            // Layout for playback controls (Rewind, Play/Pause, Skip)
            let control_layout = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3), // Rewind button
                    Constraint::Ratio(1, 3), // Play/Pause button
                    Constraint::Ratio(1, 3), // Skip button
                ])
                .split(now_playing_layout[4]);

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

            let rewind = Paragraph::new("[<< Rewind]")
                .style(rewind_style)
                .alignment(Alignment::Center)
                .add_modifier(Modifier::BOLD);

            // Dynamically set Play/Pause button text based on playback state
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

            f.render_widget(rewind, control_layout[0]);
            f.render_widget(play_pause, control_layout[1]);
            f.render_widget(skip, control_layout[2]);

            let create_track_block = Block::default()
                .title("Create New Track")
                .borders(Borders::ALL);

            let inner_create_track = create_track_block.inner(create_track_area);
            f.render_widget(create_track_block, create_track_area);

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
                    Constraint::Length(1), // Space
                    Constraint::Length(1), // Generate random button
                ])
                .split(inner_create_track);

            let params_layout_top = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4), // Cell for Scale
                    Constraint::Ratio(1, 4), // Empty cell (spacer)
                    Constraint::Ratio(1, 4), // Empty cell (spacer)
                    Constraint::Ratio(1, 4), // Cell for Style
                ])
                .split(create_track_layout[0]);

            // Style for the Scale widget, indicating focus or editing state
            let scale_style = if self.current_focus == InputId::Scale {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow) // Focused
                } else {
                    Style::default().fg(Color::Green) // Editing or popup active
                }
            } else {
                Style::default() // Not focused
            };

            let scale_widget_paragraph =
                Paragraph::new(format!("Scale: [ {} ▼]", self.state.scale))
                    .style(scale_style)
                    .add_modifier(Modifier::BOLD)
                    .alignment(Alignment::Center);

            f.render_widget(scale_widget_paragraph, params_layout_top[0]);

            let style_style = if self.current_focus == InputId::Style {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default() // Not focused
            };
            let style_param = Paragraph::new(format!("Style: [ {} ▼]", self.state.style))
                .style(style_style) // Apply conditional style
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(style_param, params_layout_top[3]);

            // Layout for BPM and Length (second row of parameters)
            let params_layout_bottom = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 4), // Cell for BPM
                    Constraint::Ratio(1, 4), // Empty cell (spacer)
                    Constraint::Ratio(1, 4), // Empty cell (spacer)
                    Constraint::Ratio(1, 4), // Cell for Length
                ])
                .split(create_track_layout[3]); // Use the second parameter row

            let bpm_style = if self.current_focus == InputId::Bpm {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else { // Editing
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let bpm = Paragraph::new(format!("BPM: [{}]", self.state.bpm))
                .style(bpm_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(bpm, params_layout_bottom[0]); // Render BPM in the first cell of the bottom params row

            let length_style = if self.current_focus == InputId::Length {
                 if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else { // Popup active
                    Style::default().fg(Color::Green)
                }
            } else {
                Style::default()
            };

            let length = Paragraph::new(format!("Length: [{} ▼]", self.state.length))
                .style(length_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(length, params_layout_bottom[3]); // Render Length in the fourth cell of the bottom params row

            let seed_style = if self.current_focus == InputId::Seed {
                if self.state.input_mode == InputMode::Navigation {
                    Style::default().fg(Color::Yellow)
                } else { // Editing
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
            f.render_widget(seed, create_track_layout[5]); // Render Seed in its dedicated row

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
            f.render_widget(generate, create_track_layout[6]); // Render Generate in its dedicated row

            let generate_style = if self.current_focus == InputId::GenerateRandom
                && self.state.input_mode == InputMode::Navigation
            {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let generate_random = Paragraph::new("[♫ Generate Random]")
                .style(generate_style)
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);
            f.render_widget(generate_random, create_track_layout[8]); // Render GenerateRandom in its dedicated row

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
                        let bpm_widget_cell_area = params_layout_bottom[0]; // Use the new layout cell for BPM
                        let full_text_content = format!("BPM: [{}]", self.state.bpm);
                        let text_prefix_len = "BPM: [".len() as u16;

                        let centered_text_start_x = bpm_widget_cell_area.x
                            + (bpm_widget_cell_area.width / 2)
                                .saturating_sub(full_text_content.len() as u16 / 2);

                        let x =
                            centered_text_start_x + text_prefix_len + self.state.bpm.len() as u16;
                        let y = bpm_widget_cell_area.y; // Use y from the cell area
                        f.set_cursor(x, y);
                    }
                    InputId::Seed => {
                        let seed_widget_row_area = create_track_layout[4]; // Row for Seed (now correct)
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

            // Help Popup / Menu
            if self.state.show_help {
                let help_text = vec![
                    Line::from(Span::styled("--- Hotkeys ---", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))),
                    Line::from(""),
                    Line::from(Span::styled("Global:", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from("  q: Quit"),
                    Line::from("  p: Play/Pause"),
                    Line::from("  r: Rewind Song"),
                    Line::from("  f: Fast Forward (New Random Song)"),
                    Line::from("  ?: Toggle Help Menu"),
                    Line::from(""),
                    Line::from(Span::styled("Navigation Mode (Arrow Keys or Vim Keys):", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from("  ↑/k: Navigate Up"),
                    Line::from("  ↓/j: Navigate Down"),
                    Line::from("  ←/h: Navigate Left"),
                    Line::from("  →/l: Navigate Right"),
                    Line::from("  Enter: Select / Activate"),
                    Line::from(""),
                    Line::from(Span::styled("Editing Mode (for BPM, Seed, Load ID):", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from("  Enter: Confirm Edit"),
                    Line::from("  Esc: Cancel Edit"),
                    Line::from("  Backspace: Delete Character"),
                    Line::from(""),
                    Line::from(Span::styled("Popup Menus (Scale, Style, Length):", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from("  ↑/k: Cycle Up"),
                    Line::from("  ↓/j: Cycle Down"),
                    Line::from("  Enter: Select Item"),
                    Line::from("  Esc: Close Popup"),
                ];

                let popup_width = 60;
                let popup_height = (help_text.len() + 2) as u16; // +2 for borders

                let popup_x = (f.size().width.saturating_sub(popup_width)) / 2;
                let popup_y = (f.size().height.saturating_sub(popup_height)) / 2;

                let popup_area = Rect {
                    x: popup_x,
                    y: popup_y,
                    width: popup_width,
                    height: popup_height,
                };

                f.render_widget(Clear, popup_area); // Clear the area for the popup

                let help_block = Block::default()
                    .title("Help - Hotkeys")
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::DarkGray));
                
                let help_paragraph = Paragraph::new(help_text)
                    .block(help_block)
                    .wrap(ratatui::widgets::Wrap { trim: true });

                f.render_widget(help_paragraph, popup_area);
            }

            // Render Help Hint Footer
            let help_hint = Paragraph::new("Press ? for help")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(help_hint, footer_area);

        })?;
        Ok(())
    }

    /* get_current_app_state - Returns a clone of the current application state.
     *
     * inputs:
     *     - &self
     *
     * outputs:
     *     - AppState : A copy of the TUI's current `AppState`.
     */
    pub fn get_current_app_state(&self) -> AppState {
        self.state.clone()
    }

    /* set_app_state - Replaces the current application state with a new one.
     *
     * inputs:
     *     - &mut self
     *     - new_state (AppState): The new state to apply.
     *
     * outputs:
     *     - None
     */
    pub fn set_app_state(&mut self, new_state: AppState) {
        self.state = new_state;
    }

    /* set_playing_state - Explicitly sets the playback state (playing or paused).
     *
     * This is used, for example, after music generation completes or a song is loaded
     * to ensure the TUI reflects the correct playback status.
     *
     * inputs:
     *     - &mut self
     *     - is_playing (bool): True to set to playing, false for paused.
     *
     * outputs:
     *     - None
     */
    pub fn set_playing_state(&mut self, is_playing: bool) {
        self.state.is_playing = is_playing;
    }

    /* toggle_help - Toggles the visibility of the help menu.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - None
     */
    pub fn toggle_help(&mut self) {
        self.state.show_help = !self.state.show_help;
    }

    /* is_paused - Checks if music playback is currently paused.
     *
     * inputs:
     *     - &self
     *
     * outputs:
     *     - bool : True if playback is paused, false otherwise.
     */
    pub fn is_paused(&self) -> bool {
        !self.state.is_playing
    }

    /* clear_song_loader_input - Clears the text from the song loader input field.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - None
     */
    pub fn clear_song_loader_input(&mut self) {
        self.state.song_loader_input.clear();
    }

    /* focus_on_play_pause - Sets the UI focus to the Play/Pause button.
     *
     * This also ensures the TUI is in Navigation mode.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - None
     */
    pub fn focus_on_play_pause(&mut self) {
        self.current_focus = InputId::PlayPause;
        self.state.input_mode = InputMode::Navigation; // Ensure navigation mode after focusing.
    }

    /* show_song_id_error - Displays an error message related to song ID loading.
     *
     * Sets the TUI to `SongIdErrorPopup` mode to show the message.
     *
     * inputs:
     *     - &mut self
     *     - error_message (String): The error message to display.
     *
     * outputs:
     *     - None
     */
    pub fn show_song_id_error(&mut self, error_message: String) {
        self.state.song_id_error = Some(error_message);
        self.state.input_mode = InputMode::SongIdErrorPopup;
    }

    /* reset_current_song_progress - Resets playback progress for the current song (e.g., on rewind).
     *
     * This visually resets the elapsed time and progress bar to the beginning.
     * The total song duration remains unchanged.
     * Typically, playback is set to `true` after a rewind.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - None
     */
    pub fn reset_current_song_progress(&mut self) {
        // Only reset the current playback position visually.
        // The actual duration and definitive progress comes from music_service.
        // The existing current_song_duration_secs remains, so "MM:SS / TotalDuration" looks consistent.
        self.state.current_song_elapsed_secs = 0.0;
        self.state.current_song_progress = 0.0;
        self.state.is_playing = true; // Ensure playing state is true after rewind.
    }

    /* reset_progress_for_new_song - Resets all progress information for a new song.
     *
     * Calls `update_progress(0,0)` to clear times and progress percentage.
     * Note: Setting `is_playing` or clearing `current_song_id_display` is typically
     * handled by the main application logic when a new song starts.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - None
     */
    pub fn reset_progress_for_new_song(&mut self) {
        self.update_progress(0, 0);
        // self.state.current_song_id_display = None; // Clearing ID is handled by main.rs/progress updates
    }

    /* handle_input - Processes user input events from the terminal.
     *
     * This method polls for keyboard events. Based on the current `InputMode`
     * (e.g., Navigation, Editing, Popup) and the specific key pressed, it determines
     * the appropriate `UserAction` to return. It handles global shortcuts (like Quit, ToggleHelp),
     * navigation between UI elements, text input into fields, interaction with popups,
     * and actions related to music control and generation.
     *
     * inputs:
     *     - &mut self
     *
     * outputs:
     *     - std::io::Result<UserAction> : The determined `UserAction` or an I/O error.
     */
    pub fn handle_input(&mut self) -> std::io::Result<UserAction> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if self.state.show_help {
                    // When help is shown, only '?' or 'q' on press do something.
                    // All other events (other keys, or non-press events) are NoOp.
                    if key.kind == event::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('?') => return Ok(UserAction::ToggleHelp), // Action to close help
                            KeyCode::Char('q') => return Ok(UserAction::Quit),
                            _ => {} // Other pressed keys will fall through to the NoOp below
                        }
                    }
                    return Ok(UserAction::NoOp); // Catch-all for any event if help is shown and not handled above
                }

                // ---- Help is NOT shown at this point ----
                // Global keybindings (available when help is NOT shown)
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('?') => return Ok(UserAction::ToggleHelp), // Action to open help
                        KeyCode::Char('q') => return Ok(UserAction::Quit),
                        KeyCode::Char('p') => return Ok(UserAction::TogglePlayback),
                        KeyCode::Char('r') => return Ok(UserAction::RewindSong),
                        KeyCode::Char('f') => return Ok(UserAction::FastForwardSong),
                        _ => {} 
                    }
                }

                match self.state.input_mode {
                    InputMode::Navigation => {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.current_focus = next_focus(self.current_focus, Direction::Up);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Down);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Left);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                self.current_focus =
                                    next_focus(self.current_focus, Direction::Right);
                                Ok(UserAction::Navigate)
                            }
                            KeyCode::Enter => match self.current_focus {
                                InputId::Rewind => Ok(UserAction::RewindSong),
                                InputId::PlayPause => {
                                    Ok(UserAction::TogglePlayback)
                                }
                                InputId::Skip => Ok(UserAction::FastForwardSong),
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
                                InputId::GenerateRandom => Ok(UserAction::GenerateRandomMusic),
                                InputId::SongLoader => {
                                    // Added SongLoader Enter in Navigation mode
                                    self.editing_original_value =
                                        Some(self.state.song_loader_input.clone());
                                    self.state.input_mode = InputMode::SongLoaderEditing;
                                    Ok(UserAction::SwitchToEditing)
                                }
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

/* next_focus - Determines the next UI element to focus on based on navigation direction.
 *
 * Given the currently focused element (`current`) and a navigation `Direction`,
 * this function consults the `INPUT_GRAPH` to find the `InputId` of the
 * neighboring element in that direction. If no neighbor exists in the given
 * direction, focus remains on the current element.
 *
 * inputs:
 *     - current (InputId): The `InputId` of the currently focused UI element.
 *     - direction (Direction): The direction of navigation.
 *
 * outputs:
 *     - InputId : The `InputId` of the next element to focus, or the current one if no move is possible.
 */
fn next_focus(current: InputId, direction: Direction) -> InputId {
    let graph = get_input_graph();
    graph
        .get(&current)
        .and_then(|node| node.neighbors.get(&direction).copied())
        .unwrap_or(current) // If no neighbor, stay on the current input
}
