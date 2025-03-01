extern crate rust_music_theory as rustmt;
use crate::rustmt::chord::{Chord, Number as ChordNumber, Quality as ChordQuality};
use crate::rustmt::note::{Note, Notes, PitchClass};
use dasp_signal::Signal;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink, Source};
use std::io::{stdin, Read};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Convert a Note to its MIDI number.
/// MIDI numbers represent notes in a standardized way where each number is a specific pitch.
fn note_to_midi(note: &Note) -> i32 {
    // Get the semitone offset based on the pitch class (like C, D, E, etc.)
    let semitone = match note.pitch_class {
        PitchClass::C => 0,
        PitchClass::Cs => 1,  // C sharp
        PitchClass::D => 2,
        PitchClass::Ds => 3,  // D sharp
        PitchClass::E => 4,
        PitchClass::F => 5,
        PitchClass::Fs => 6,  // F sharp
        PitchClass::G => 7,
        PitchClass::Gs => 8,  // G sharp
        PitchClass::A => 9,
        PitchClass::As => 10, // A sharp
        PitchClass::B => 11,
    };
    
    // Calculate MIDI number based on octave and semitone
    // Formula: (octave+1) * 12 + semitone
    ((note.octave + 1) * 12 + semitone) as i32
}

/// Convert a Note to its frequency in Hz.
/// This uses the standard formula to convert MIDI note numbers to frequency.
fn note_to_frequency(note: &Note) -> f32 {
    let midi_number = note_to_midi(note) as f32;
    
    // Standard formula to convert MIDI note to frequency:
    // A4 (MIDI 69) = 440Hz, and each semitone is a factor of 2^(1/12)
    440.0 * 2f32.powf((midi_number - 69.0) / 12.0)
}

/// Generate chord samples.
/// This creates the sound data for a chord with the given properties.
fn generate_chord_samples(
    root_note: PitchClass,         // The root note of the chord (C, D, etc.)
    chord_quality: ChordQuality,   // Major, minor, diminished, etc.
    chord_type: ChordNumber,       // Triad, seventh, ninth, etc.
    duration_seconds: u32,         // How long the chord should play
    sample_rate: u32,              // Audio quality (samples per second)
) -> Vec<f32> {
    // Create a chord object using the music theory library
    let chord = Chord::new(root_note, chord_quality, chord_type);
    
    // Get the actual notes in the chord
    let chord_notes = chord.notes();

    // Print information about the chord
    println!(
        "Generating chord: {:?} {:?} {:?}",
        root_note, chord_quality, chord_type
    );
    println!("Chord notes: {:?}", chord_notes);

    // Calculate the frequency for each note in the chord
    let note_frequencies: Vec<f32> = chord_notes.iter().map(note_to_frequency).collect();

    // Generate sine wave signals for each frequency
    let mut note_generators: Vec<_> = note_frequencies
        .iter()
        .map(|&freq| {
            dasp_signal::rate(sample_rate as f64)  // Set the sample rate
                .const_hz(freq as f64)            // Create a constant frequency
                .sine()                           // Generate a sine wave
                .map(|x| (x * 0.3) as f32)        // Reduce amplitude to avoid distortion
        })
        .collect();

    // Calculate the total number of samples needed
    let total_samples = sample_rate as usize * duration_seconds as usize;
    let mut chord_samples = Vec::with_capacity(total_samples);

    // Combine samples from all notes to create the chord sound
    for _ in 0..total_samples {
        // Sum all the sine waves together
        let sample_sum: f32 = note_generators.iter_mut().map(|sine| sine.next()).sum();
        
        // Average the samples to avoid clipping
        chord_samples.push(sample_sum / note_frequencies.len() as f32);
    }

    chord_samples
}

/// Mix multiple sample vectors together.
/// This combines multiple sounds with different volumes.
fn mix_samples(sample_collections: Vec<Vec<f32>>, volume_levels: &[f32]) -> Vec<f32> {
    // Return empty vector if there's nothing to mix
    if sample_collections.is_empty() {
        return Vec::new();
    }

    // Find the longest sample vector
    let max_length = sample_collections.iter().map(|s| s.len()).max().unwrap();
    let mut combined_samples = vec![0.0; max_length];

    // Mix all samples together with their respective volumes
    for (i, samples) in sample_collections.iter().enumerate() {
        // Get the volume level for this sample set (default to 1.0)
        let volume = *volume_levels.get(i).unwrap_or(&1.0);

        // Add each sample multiplied by its volume
        for (j, &sample) in samples.iter().enumerate() {
            if j < max_length {
                combined_samples[j] += sample * volume;
            }
        }
    }

    // Normalize the result to prevent distortion
    let max_amplitude = combined_samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
    if max_amplitude > 1.0 {
        for sample in &mut combined_samples {
            *sample /= max_amplitude;
        }
    }

    combined_samples
}

/// Capture a single character from stdin without requiring Enter to be pressed
fn read_char() -> Option<char> {
    // Create a 1-byte buffer to store the character
    let mut buffer = [0; 1];
    
    // Try to read exactly one byte from standard input
    match stdin().read_exact(&mut buffer) {
        Ok(_) => Some(buffer[0] as char),  // Convert the byte to a character
        Err(_) => None,                    // Return None if reading failed
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
    let sample_rate = 44100;         // CD-quality audio (44.1 kHz)
    let chord_duration = 3;          // Duration of each chord segment in seconds

    // Generate the first chord: C major (I chord)
    println!("Generating C Major chord...");
    let c_major_samples = generate_chord_samples(
        PitchClass::C,               // Root note C
        ChordQuality::Major,         // Major chord
        ChordNumber::Triad,          // Three notes (root, third, fifth)
        chord_duration,              // Duration in seconds
        sample_rate,                 // Audio quality
    );

    // Generate the second chord: F major (IV chord, harmonizes well with C)
    println!("Generating F Major chord...");
    let f_major_samples = generate_chord_samples(
        PitchClass::F,               // Root note F
        ChordQuality::Major,         // Major chord
        ChordNumber::Triad,          // Three notes
        chord_duration,
        sample_rate,
    );

    // Mix the two chords together
    println!("Mixing chord pair...");
    let chords_to_mix = vec![c_major_samples.clone(), f_major_samples.clone()];
    let chord_volumes = vec![0.5, 0.5];  // Equal volume for both chords
    let combined_chord_samples = mix_samples(chords_to_mix, &chord_volumes);

    // Create a sequence of all sounds: C major, F major, then both together
    let mut audio_sequence = Vec::new();
    audio_sequence.extend_from_slice(&c_major_samples);  // Add C major chord

    // Add a short silence between segments
    let silence_duration_samples = (sample_rate as f32 * 0.3) as usize; // 300ms
    let silence_samples = vec![0.0; silence_duration_samples];  // Silence is represented by zeros
    audio_sequence.extend_from_slice(&silence_samples);

    // Add F major chord
    audio_sequence.extend_from_slice(&f_major_samples);
    audio_sequence.extend_from_slice(&silence_samples);
    
    // Add the mixed chord (both played together)
    audio_sequence.extend_from_slice(&combined_chord_samples);
    audio_sequence.extend_from_slice(&silence_samples);

    // Set up audio output system
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to get output stream");

    // Create shared state variables for thread communication
    // Arc = Atomic Reference Counting (allows sharing between threads)
    // Mutex = Mutual Exclusion (ensures only one thread can modify at a time)
    let is_playback_paused = Arc::new(Mutex::new(false));  // Tracks if playback is paused
    let should_continue_running = Arc::new(Mutex::new(true));  // Tracks if program should continue running

    // Clone the Arcs for the input thread
    // This creates new references to the same shared data
    let is_playback_paused_for_input = Arc::clone(&is_playback_paused);
    let should_continue_running_for_input = Arc::clone(&should_continue_running);

    // Create a thread for handling keyboard input
    let input_thread = thread::spawn(move || {
        println!("Press SPACE to pause/resume playback");
        println!("Press 'q' to quit");

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
            thread::sleep(Duration::from_millis(10));

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
        
        // Wait for the input thread to finish
        input_thread.join().unwrap();

        // Make sure we're marked as not running
        *should_continue_running.lock().unwrap() = false;
    });

    // Wait for the playback thread to finish
    playback_thread.join().unwrap();

    println!("Playback ended.");
}
