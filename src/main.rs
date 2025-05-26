mod bass;
// mod config; // Removed
mod gen;
mod melodies;
mod progs;
mod tui;

use crate::gen::MusicControl;
use crate::tui::UserAction;
use std::error::Error;
use std::thread::JoinHandle; // For managing the music service thread
use ratatui::prelude::CrosstermBackend; // Added for TUI backend
use std::thread; // Added for thread::sleep
use std::time::Duration; // Added for Duration
// Simplified crossbeam import if Receiver is not directly used here for an initialized variable
use crossbeam_channel::{self, Sender as CrossbeamSender}; 

fn main() -> Result<(), Box<dyn Error>> {
    // music_control_receiver was unused
    let (music_control_sender, _) = crossbeam_channel::unbounded::<MusicControl>();
    let (progress_sender, progress_receiver) = crossbeam_channel::unbounded::<gen::MusicProgress>();

    let mut tui = tui::Tui::new(CrosstermBackend::new(std::io::stdout()))?;
    tui.setup()?;

    let mut music_service_handle: Option<JoinHandle<()>> = None;
    let mut music_sender_option: Option<CrossbeamSender<MusicControl>> = Some(music_control_sender.clone());

    // Initial music generation (optional - could start paused or with a default track)
    // For now, let's not auto-start. User must press Play or Generate.
    // let initial_app_state = tui.get_current_app_state();
    // gen::run_music_service(initial_app_state, music_control_receiver, progress_sender.clone()); // Pass progress_sender
    // music_service_handle = Some(std::thread::spawn(|| {})); // This handle logic needs to be more robust

    'main: loop {
        tui.draw()?;

        // Check for progress updates from the music service
        if let Ok(progress) = progress_receiver.try_recv() { // try_recv from crossbeam
            tui.update_progress(progress.current_samples, progress.total_samples, progress.actual_seed);
        }

        match tui.handle_input()? {
            UserAction::Quit => {
                if let Some(sender) = music_sender_option.take() {
                    sender.send(MusicControl::Terminate).ok(); // send from crossbeam
                }
                if let Some(handle) = music_service_handle.take() {
                    handle.join().expect("Failed to join music service thread");
                }
                break 'main;
            }
            UserAction::TogglePlayback => {
                if let Some(sender) = &music_sender_option {
                    if tui.is_paused() { 
                        let _ = sender.send(MusicControl::Resume); // send from crossbeam
                        tui.set_playing_state(true); 
                    } else {
                        let _ = sender.send(MusicControl::Pause); // send from crossbeam
                        tui.set_playing_state(false); 
                    }
                }
            }
            UserAction::GenerateMusic => {
                if let Some(sender) = music_sender_option.take() {
                    let _ = sender.send(MusicControl::Terminate);
                    if let Some(handle) = music_service_handle.take() { 
                        handle.join().expect("Failed to join music thread");
                    }
                }
                let app_state_clone = tui.get_current_app_state();
                // Create new crossbeam channels for the new service
                let (new_music_sender, new_music_receiver) = crossbeam_channel::unbounded::<MusicControl>();
                let new_progress_sender_clone = progress_sender.clone(); // This is now a CrossbeamSender clone

                music_sender_option = Some(new_music_sender.clone());
                music_service_handle = Some(thread::spawn(move || { 
                    // Corrected call: 3 arguments, new_music_receiver is CrossbeamReceiver, new_progress_sender_clone is CrossbeamSender
                    gen::run_music_service(app_state_clone, new_music_receiver, new_progress_sender_clone);
                }));
                tui.set_playing_state(true);
                if let Some(sender) = &music_sender_option {
                    let _ = sender.send(MusicControl::Resume);
                }
            }
            UserAction::ToggleLoop => {
                // For now, main.rs doesn't need to do anything specific with loop state
                // as it's managed by the TUI and music generation logic might use AppState.is_loop_enabled directly if needed.
                // Or, a MusicControl message could be introduced if the music service needs to know about loop changes.
            }
            UserAction::NoOp => {}
            // Ensure other UserActions that tui might return but main doesn't explicitly handle are covered or lead to NoOp
            UserAction::UpdateInput |
            UserAction::Navigate |
            UserAction::SwitchToEditing |
            UserAction::SwitchToNavigation |
            UserAction::OpenPopup |
            UserAction::CyclePopupOption |
            UserAction::SelectPopupItem => { /* These are handled by TUI state changes, main loop continues */ }
        }

        thread::sleep(Duration::from_millis(16));
    }

    tui.teardown()?;
    Ok(())
}

