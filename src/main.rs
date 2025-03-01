mod progs;

use std::io::stdin;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink, Source};

fn play_progression(
    prog_name: String,
    root_note: u8,
    chord_duration: f32,
    mut audio_sequence: Vec<f32>,
) -> Vec<f32> {
    println!("Playing progression...");
    // Get the progression chords
    let progression = progs::get_progression(prog_name, root_note, chord_duration);

    // Short silence between segments
    let silence_duration_samples = (44100.0 * 0.0) as usize; // 300ms
    let silence_samples = vec![0.0; silence_duration_samples]; // Silence is represented by zeros

    // Add each chord to the sequence with silence in between
    for chord in progression {
        audio_sequence.extend_from_slice(&chord);
        audio_sequence.extend_from_slice(&silence_samples);
    }

    // Return the modified sequence
    audio_sequence
}

fn read_char() -> Option<char> {
    // Create a 1-byte buffer to store the character
    let mut buffer = [0; 1];

    // Try to read exactly one byte from standard input
    match stdin().read_exact(&mut buffer) {
        Ok(_) => Some(buffer[0] as char), // Convert the byte to a character
        Err(_) => None,                   // Return None if reading failed
    }
}

/// Set terminal to raw mode (no line buffering)
/// This allows reading single keypresses without requiring Enter
fn set_raw_mode() {
    // For non-Windows systems (Unix-like)
    #[cfg(not(windows))]
    {
        use std::process::Command;
        // Execute the 'stty raw' command to set terminal to raw mode
        Command::new("stty").arg("raw").status().unwrap();
    }
}

/// Restore normal terminal mode
fn reset_terminal() {
    // For non-Windows systems (Unix-like)
    #[cfg(not(windows))]
    {
        use std::process::Command;
        // Execute the 'stty -raw' command to return terminal to normal mode
        Command::new("stty").arg("-raw").status().unwrap();
    }
}

fn main() {
    // Audio settings
    let sample_rate = 44100; // CD-quality audio (44.1 kHz)
    let chord_duration = 2.0; // Duration of each chord segment in seconds

    // Available progressions
    println!("Chord Progression Player");
    println!("========================");
    println!("Available progressions:");
    println!("1. Blues progression (I-IV-V-IV)");
    println!("2. Pop progression (I-V-vi-IV)");
    println!("3. Jazz progression (ii-V-I)");
    println!("4. Simple progression (I-IV)");
    println!("\nEnter choice (1-4): ");

    // Read user choice
    let mut choice = String::new();
    stdin().read_line(&mut choice).expect("Failed to read line");
    let choice = choice.trim();

    // Read root note
    println!("\nEnter root note (0-11, where 0=C, 1=C#, 2=D, etc.): ");
    let mut root_note = String::new();
    stdin()
        .read_line(&mut root_note)
        .expect("Failed to read line");
    let root_note: u8 = root_note.trim().parse().unwrap_or(0);

    // Create our audio sequence based on user choice
    let mut audio_sequence = Vec::new();

    audio_sequence = match choice {
        "1" => play_progression(
            String::from("blues"),
            root_note,
            chord_duration,
            audio_sequence,
        ),
        "2" => play_progression(
            String::from("pop"),
            root_note,
            chord_duration,
            audio_sequence,
        ),
        "3" => play_progression(
            String::from("jazz"),
            root_note,
            chord_duration,
            audio_sequence,
        ),
        _ => play_progression(
            String::from("default"),
            root_note,
            chord_duration,
            audio_sequence,
        ),
    };

    // Set up audio output system
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to get output stream");

    // Create shared state variables for thread communication
    let is_playback_paused = Arc::new(Mutex::new(false)); // Tracks if playback is paused
    let should_continue_running = Arc::new(Mutex::new(true)); // Tracks if program should continue running

    // Clone the Arcs for the input thread
    let is_playback_paused_for_input = Arc::clone(&is_playback_paused);
    let should_continue_running_for_input = Arc::clone(&should_continue_running);

    // Create a thread for handling keyboard input
    let input_thread = thread::spawn(move || {
        println!("\nControls:");
        println!("- Press SPACE to pause/resume playback");
        println!("- Press 'q' to quit");

        // Set terminal to raw mode for single keypress detection
        set_raw_mode();

        loop {
            // Try to read a single keypress
            if let Some(key) = read_char() {
                match key {
                    // Space bar toggles pause/play
                    ' ' => {
                        // Lock the mutex to safely access the shared data
                        let mut playback_state = is_playback_paused_for_input.lock().unwrap();
                        // Toggle the paused state
                        *playback_state = !*playback_state;

                        // Print appropriate message
                        if *playback_state {
                            println!("\nPlayback paused. Press SPACE to resume.");
                        } else {
                            println!("\nPlayback resumed. Press SPACE to pause.");
                        }
                    }
                    // 'q' key quits the program
                    'q' => {
                        println!("\nExiting...");
                        // Set running to false to signal other threads to stop
                        *should_continue_running_for_input.lock().unwrap() = false;
                        break;
                    }
                    // Ignore other keypresses
                    _ => {}
                }
            }

            // Small sleep to avoid consuming too much CPU
            thread::sleep(Duration::from_millis(1));

            // Check if we should exit the loop
            if !*should_continue_running_for_input.lock().unwrap() {
                break;
            }
        }

        // Reset terminal mode when done
        reset_terminal();
    });

    // Create a thread for audio playback
    let playback_thread = thread::spawn(move || {
        // Continue playing as long as should_continue_running is true
        while *should_continue_running.lock().unwrap() {
            // Create a new audio sink (output)
            let audio_sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

            // Prepare audio data and set it to repeat
            let audio_source =
                SamplesBuffer::new(1, sample_rate, audio_sequence.clone()).repeat_infinite();

            // Add the audio to the sink
            audio_sink.append(audio_source);

            println!("Starting playback loop. Press SPACE to pause/resume.");

            // Continue until the sink is empty or we're told to stop
            while !audio_sink.empty() && *should_continue_running.lock().unwrap() {
                // Check if playback should be paused
                if *is_playback_paused.lock().unwrap() {
                    audio_sink.pause();
                } else {
                    audio_sink.play();
                }

                // Small sleep to avoid consuming too much CPU
                thread::sleep(Duration::from_millis(100));
            }

            // Stop the sink if we're still running
            if *should_continue_running.lock().unwrap() {
                audio_sink.stop();
            }
        }
    });

    // Wait for the playback thread to finish
    if let Err(e) = playback_thread.join() {
        eprintln!("Error joining playback thread: {:?}", e);
    }

    // Wait for the input thread to finish
    if let Err(e) = input_thread.join() {
        eprintln!("Error joining input thread: {:?}", e);
    }

    println!("Playback ended.");
}

