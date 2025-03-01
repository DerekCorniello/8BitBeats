mod progs;

use std::io::stdin;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink, Source};
use rust_music_theory::chord::{Number as ChordNumber, Quality as ChordQuality};

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
    println!("5. Custom C-F progression (with mixed chords)");
    println!("\nEnter choice (1-5): ");

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

    // Short silence between segments
    let silence_duration_samples = (sample_rate as f32 * 0.3) as usize; // 300ms
    let silence_samples = vec![0.0; silence_duration_samples]; // Silence is represented by zeros

    match choice {
        "1" => {
            println!("Playing blues progression...");
            // Get the blues progression chords
            let progression =
                progs::get_progression("blues".to_string(), root_note, chord_duration);

            // Add each chord to the sequence with silence in between
            for chord in progression {
                audio_sequence.extend_from_slice(&chord);
                audio_sequence.extend_from_slice(&silence_samples);
            }
        }
        "2" => {
            println!("Playing pop progression...");
            let progression = progs::get_progression("pop".to_string(), root_note, chord_duration);

            for chord in progression {
                audio_sequence.extend_from_slice(&chord);
                audio_sequence.extend_from_slice(&silence_samples);
            }
        }
        "3" => {
            println!("Playing jazz progression...");
            let progression = progs::get_progression("jazz".to_string(), root_note, chord_duration);

            for chord in progression {
                audio_sequence.extend_from_slice(&chord);
                audio_sequence.extend_from_slice(&silence_samples);
            }
        }
        "4" => {
            println!("Playing simple I-IV progression...");
            let progression =
                progs::get_progression("default".to_string(), root_note, chord_duration);

            for chord in progression {
                audio_sequence.extend_from_slice(&chord);
                audio_sequence.extend_from_slice(&silence_samples);
            }
        }
        "5" | _ => {
            println!("Playing custom C-F progression with mixed chords...");

            // Generate chord samples using the root note the user provided
            let root_chord_samples = progs::generate_chord_samples(
                progs::get_pitch(root_note),
                ChordQuality::Major,
                ChordNumber::Triad,
                chord_duration,
                sample_rate,
            );

            // Generate fourth chord (five semitones up from root)
            let fourth_chord_samples = progs::generate_chord_samples(
                progs::get_pitch(root_note + 5),
                ChordQuality::Major,
                ChordNumber::Triad,
                chord_duration,
                sample_rate,
            );

            // Mix them for the third part
            let chords_to_mix = vec![root_chord_samples.clone(), fourth_chord_samples.clone()];
            let chord_volumes = vec![0.5, 0.5]; // Equal volume for both chords
            let combined_chord_samples = progs::mix_samples(chords_to_mix, &chord_volumes);

            // Create sequence: root chord, fourth chord, then both together
            audio_sequence.extend_from_slice(&root_chord_samples);
            // audio_sequence.extend_from_slice(&silence_samples);
            audio_sequence.extend_from_slice(&fourth_chord_samples);
            // audio_sequence.extend_from_slice(&silence_samples);
            audio_sequence.extend_from_slice(&combined_chord_samples);
            // audio_sequence.extend_from_slice(&silence_samples);
        }
    }

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

    // Make sure we're marked as not running

    // Wait for the input thread to finish
    if let Err(e) = input_thread.join() {
        eprintln!("Error joining input thread: {:?}", e);
    }

    println!("Playback ended.");
}

