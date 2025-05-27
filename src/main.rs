mod bass;
// mod config; // Removed
mod gen;
mod melodies;
mod progs;
mod tui;

use crate::gen::MusicControl;
use crate::gen::parse_song_id_to_app_state; // For parsing song IDs
use crate::tui::UserAction;
use std::error::Error;
use std::thread::JoinHandle; // For managing threads
use ratatui::prelude::CrosstermBackend; // For TUI backend
use std::thread; // For threading capabilities
use std::time::Duration; // For time-based operations
use crossbeam_channel::Sender as CrossbeamSender; // For sending messages between threads

/* main - Initializes the TUI and music service, then enters the main event loop.
 *
 * This function is the entry point of the 8BitBeats application. It sets up
 * the terminal user interface (TUI), initializes channels for communication
 * between the TUI and the music generation service, and then enters a loop
 * to handle user input and update the TUI.
 *
 * inputs:
 *     - None
 *
 * outputs
 *     - Result<(), Box<dyn Error>> : Ok if the application runs and exits successfully,
 *                                   or an error if an unrecoverable issue occurs.
 */
fn main() -> Result<(), Box<dyn Error>> {
    let (music_control_sender, _music_control_receiver) = crossbeam_channel::unbounded::<MusicControl>();
    let (progress_sender, progress_receiver) = crossbeam_channel::unbounded::<gen::MusicProgress>();

    let mut tui = tui::Tui::new(CrosstermBackend::new(std::io::stdout()))?;
    tui.setup()?;

    let mut music_service_handle: Option<JoinHandle<()>> = None;
    let mut music_sender_option: Option<CrossbeamSender<MusicControl>> = Some(music_control_sender.clone());

    // Music generation is now manually triggered by the user.
    'main: loop {
        tui.draw()?;

        // Check for progress updates from the music service
        if let Ok(progress) = progress_receiver.try_recv() {
            tui.update_progress(progress.current_samples, progress.total_samples);
            
            // If a song was just generated, its ID display might not be set yet.
            // We use the actual_seed from progress to form it.
            if tui.get_current_app_state().current_song_id_display.is_none() && progress.total_samples > 0 {
                let current_app_params = tui.get_current_app_state();
                let length_part = current_app_params.length.split_whitespace().next().unwrap_or("?");
                let generated_id_str = format!("{}-{}-{}-{}-{}", 
                    current_app_params.scale, 
                    current_app_params.style, 
                    current_app_params.bpm, 
                    length_part, 
                    progress.actual_seed
                );
                tui.set_current_song_id_display(Some(generated_id_str));
            } else if progress.total_samples == 0 { // Song ended or was terminated
                tui.set_current_song_id_display(None);
            }
        }

        match tui.handle_input()? {
            UserAction::Quit => break 'main,
            UserAction::RewindSong => {
                if let Some(sender) = &music_sender_option {
                    let _ = sender.send(MusicControl::Rewind);
                    // After sending Rewind, TUI needs to be updated to reflect the song at the beginning
                    tui.reset_current_song_progress(); // Visually reset progress in TUI
                    tui.set_playing_state(true); // Ensure TUI shows as playing
                    tui.focus_on_play_pause(); // Set focus back to play/pause
                }
            }
            UserAction::FastForwardSong => {
                if let Some(sender) = music_sender_option.take() {
                    let _ = sender.send(MusicControl::Terminate);
                    if let Some(handle) = music_service_handle.take() { 
                        handle.join().expect("Failed to join music thread for fast-forward");
                    }
                }
                // Drain any lingering progress messages from the old song
                while progress_receiver.try_recv().is_ok() {}

                tui.reset_progress_for_new_song(); 
                tui.set_current_song_id_display(None); // Clear old song ID immediately
                let mut app_state_clone = tui.get_current_app_state();
                app_state_clone.seed = "".to_string(); // Ensure a new random seed is used
                // Clear progress fields in the clone to ensure gen_music_service starts fresh
                app_state_clone.current_song_progress = 0.0;
                app_state_clone.current_song_elapsed_secs = 0.0;
                app_state_clone.current_song_duration_secs = 0.0;
                
                let (new_music_sender, new_music_receiver) = crossbeam_channel::unbounded::<MusicControl>();
                let new_progress_sender_clone = progress_sender.clone();

                music_sender_option = Some(new_music_sender.clone());
                music_service_handle = Some(thread::spawn(move || { 
                    gen::run_music_service(app_state_clone, new_music_receiver, new_progress_sender_clone);
                }));
                tui.set_playing_state(true); // Set TUI to playing
                tui.focus_on_play_pause(); // Optional: set focus to Play/Pause
            }
            UserAction::GenerateMusic => {
                if let Some(sender) = music_sender_option.take() {
                    let _ = sender.send(MusicControl::Terminate);
                    if let Some(handle) = music_service_handle.take() { 
                        handle.join().expect("Failed to join music thread");
                    }
                }
                // Drain any lingering progress messages from the old song
                while progress_receiver.try_recv().is_ok() {}

                tui.reset_progress_for_new_song(); 
                tui.set_current_song_id_display(None); // Clear old song ID immediately
                let mut app_state_clone = tui.get_current_app_state(); // Make mutable
                // Clear progress fields in the clone to ensure gen_music_service starts fresh
                app_state_clone.current_song_progress = 0.0;
                app_state_clone.current_song_elapsed_secs = 0.0;
                app_state_clone.current_song_duration_secs = 0.0;

                let (new_music_sender, new_music_receiver) = crossbeam_channel::unbounded::<MusicControl>();
                let new_progress_sender_clone = progress_sender.clone(); 

                music_sender_option = Some(new_music_sender.clone());
                music_service_handle = Some(thread::spawn(move || { 
                    gen::run_music_service(app_state_clone, new_music_receiver, new_progress_sender_clone);
                }));
                tui.set_playing_state(true);
                tui.focus_on_play_pause(); 
            }
            UserAction::TogglePlayback => {
                if let Some(sender) = &music_sender_option {
                    if tui.is_paused() { // If TUI thinks it's paused, we want to play
                        let _ = sender.send(MusicControl::Resume);
                        tui.set_playing_state(true); // Update TUI state
                    } else { // If TUI thinks it's playing, we want to pause
                        let _ = sender.send(MusicControl::Pause);
                        tui.set_playing_state(false); // Update TUI state
                    }
                }
            }
            UserAction::ToggleHelp => {
                tui.toggle_help();
            }
            UserAction::AttemptLoadSong => {
                let song_name_to_load = tui.get_current_app_state().song_loader_input.trim().to_string();
                if !song_name_to_load.is_empty() {
                    match parse_song_id_to_app_state(&song_name_to_load) {
                        Ok(loaded_app_state) => {
                            // Terminate existing music service if any
                            if let Some(sender) = music_sender_option.take() {
                                let _ = sender.send(MusicControl::Terminate);
                                if let Some(handle) = music_service_handle.take() {
                                    handle.join().expect("Failed to join music thread for song load");
                                }
                            }
                             // Drain any lingering progress messages
                            while progress_receiver.try_recv().is_ok() {}
                            
                            tui.reset_progress_for_new_song(); // Reset visual progress
                            // Update TUI with loaded state, but preserve some dynamic states like is_playing
                            // parse_song_id_to_app_state returns a full AppState, so we use it directly
                            // For now, we will directly use the loaded state, this implies song starts paused
                            // and user has to press play.
                            tui.set_app_state(loaded_app_state.clone()); // Directly set TUI state
                            tui.set_current_song_id_display(Some(song_name_to_load.clone())); // Show the ID being loaded

                            let (new_music_sender, new_music_receiver) = crossbeam_channel::unbounded::<MusicControl>();
                            let new_progress_sender_clone = progress_sender.clone();
                            music_sender_option = Some(new_music_sender.clone());
                            
                            // Spawn new music service with the loaded state
                            music_service_handle = Some(thread::spawn(move || {
                                gen::run_music_service(loaded_app_state, new_music_receiver, new_progress_sender_clone);
                            }));
                            
                            // After successfully setting up the new song, send a Resume command to start it.
                            if let Some(sender) = &music_sender_option {
                                let _ = sender.send(MusicControl::Resume);
                            }
                            tui.set_playing_state(true);
                            tui.focus_on_play_pause(); 
                            tui.clear_song_loader_input();
                        }
                        Err(error_message) => {
                            tui.show_song_id_error(error_message);
                            tui.set_current_song_id_display(None); // Clear display on error
                        }
                    }
                }
            }
            UserAction::NoOp => {}
            // UserActions handled by TUI state changes or that trigger TUI updates,
            // allowing the main loop to continue.
            UserAction::UpdateInput |
            UserAction::Navigate |
            UserAction::SwitchToEditing |
            UserAction::SwitchToNavigation |
            UserAction::OpenPopup |
            UserAction::CyclePopupOption |
            UserAction::CloseSongIdErrorPopup |
            UserAction::SelectPopupItem => { /* These are handled by TUI state changes or main initiates TUI change, main loop continues */ }
        }

        thread::sleep(Duration::from_millis(16));
    }

    tui.teardown()?;
    Ok(())
}

