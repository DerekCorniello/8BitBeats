use dasp_signal::Signal;
use rand::prelude::*;
use rand::rngs::StdRng;
use rust_music_theory::note::{Note, Notes, PitchClass};
use rust_music_theory::scale::{Direction, Mode, Scale, ScaleType};

/// Convert a PitchClass to its semitone offset (0-11)
fn pitch_to_semitone(pitch: &PitchClass) -> u8 {
    match pitch {
        PitchClass::C => 0,
        PitchClass::Cs => 1,
        PitchClass::D => 2,
        PitchClass::Ds => 3,
        PitchClass::E => 4,
        PitchClass::F => 5,
        PitchClass::Fs => 6,
        PitchClass::G => 7,
        PitchClass::Gs => 8,
        PitchClass::A => 9,
        PitchClass::As => 10,
        PitchClass::B => 11,
    }
}

/// Convert a numeric value to PitchClass
fn semitone_to_pitch(semitone: u8) -> PitchClass {
    match semitone % 12 {
        0 => PitchClass::C,
        1 => PitchClass::Cs,
        2 => PitchClass::D,
        3 => PitchClass::Ds,
        4 => PitchClass::E,
        5 => PitchClass::F,
        6 => PitchClass::Fs,
        7 => PitchClass::G,
        8 => PitchClass::Gs,
        9 => PitchClass::A,
        10 => PitchClass::As,
        11 => PitchClass::B,
        _ => unreachable!(),
    }
}

/// Convert a Note to its frequency in Hz
fn note_to_frequency(note: &Note) -> f32 {
    let octave_offset = (note.octave as i32 + 1) * 12;
    let semitone = pitch_to_semitone(&note.pitch_class) as i32;
    let midi_number = octave_offset + semitone;

    // Standard formula: A4 (MIDI 69) = a440, each semitone is 2^(1/12)
    440.0 * 2f32.powf((midi_number as f32 - 69.0) / 12.0)
}

/// Represents rhythm patterns for melodies
pub enum RhythmPattern {
    // Quarter notes (1 note per beat)
    Simple,
    // Eighth notes (2 notes per beat)
    Medium,
    // Mix of eighth and sixteenth notes
    Complex,
    // Syncopated rhythm with some off-beat notes
    Syncopated,
    Swung,
}

/// Generate melody samples based on given parameters
pub fn generate_melody_samples(
    root_note: PitchClass,
    scale_type: ScaleType,
    mode: Mode,
    octave: i8,
    rhythm_pattern: RhythmPattern,
    duration_seconds: u32,
    seconds_per_quarter_note: f32,
    seed: u64,
) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    const SAMPLE_RATE: f32 = 44100.0;
    // Create scale
    let scale = Scale::new(
        scale_type, // scale type
        root_note,  // tonic
        4,          // octave
        Some(mode), // scale mode
        Direction::Ascending,
    )
    .unwrap();

    let scale_notes = scale.notes();
    let mut durations: Vec<f32> = vec![];
    let mut dur_sum = 0.0;
    // let quarter_note_duration = 60.0 / bpm as f32; // Removed, using seconds_per_quarter_note directly
                                                   // Apply rhythm pattern
    let durations = match rhythm_pattern {
        RhythmPattern::Simple => {
            // All quarter notes

            // Calculate how many quarter notes fit in the total duration
            let num_quarter_notes =
                (duration_seconds as f32 / seconds_per_quarter_note).floor() as usize; // Number of full quarter notes

            // Create a vector filled with quarter note durations
            let durations: Vec<f32> = vec![seconds_per_quarter_note; num_quarter_notes];
            durations
        }
        RhythmPattern::Medium => {
            // Mix of quarter and eighth notes

            while dur_sum < duration_seconds as f32 {
                // 50% chance of quarter note, 50% chance of eighth note
                let actual_duration = if rng.random::<bool>() { // Changed to rng.random()
                    1.0 * seconds_per_quarter_note
                } else {
                    0.5 * seconds_per_quarter_note
                };
                durations.push(actual_duration);
                dur_sum += actual_duration;
            }

            durations
        }
        RhythmPattern::Complex => {
            // Mix of quarter, eighth, and sixteenth notes
            while dur_sum < duration_seconds as f32 {
                // 25% quarter, 50% eighth, 25% sixteenth
                let roll = rng.random::<f32>(); // Changed to rng.random()
                let beat_multiplier = if roll < 0.25 {
                    1.0 // quarter
                } else if roll < 0.75 {
                    0.5 // eighth
                } else {
                    0.25 // sixteenth
                };
                let actual_duration = beat_multiplier * seconds_per_quarter_note;
                dur_sum += actual_duration;
                durations.push(actual_duration); // Push actual duration in seconds
            }

            durations
        }
        RhythmPattern::Swung => {
            // Mix of quarter and eighth notes
            // Ensure this loop also respects total duration_seconds
            dur_sum = 0.0; // Reset dur_sum for this pattern
            while dur_sum < duration_seconds as f32 {
                let dur1 = 0.66 * seconds_per_quarter_note;
                if dur_sum + dur1 > duration_seconds as f32 { break; }
                durations.push(dur1);
                dur_sum += dur1;

                let dur2 = 0.34 * seconds_per_quarter_note;
                if dur_sum + dur2 > duration_seconds as f32 { break; }
                durations.push(dur2);
                dur_sum += dur2;
            }
            durations
        }

        RhythmPattern::Syncopated => {
            // Syncopated rhythm with some off-beat notes
            // let mut durations = vec![]; // durations is already mutably borrowed

            // counts if beat is on beat or off beat
            let mut i = 0;
            dur_sum = 0.0; // Reset dur_sum for this pattern
            while dur_sum < duration_seconds as f32 {
                let beat_multiplier = if i % 2 == 0 {
                    // On-beat notes are usually shorter
                    if rng.random::<bool>() { // Changed to rng.random()
                        0.5
                    } else {
                        0.25
                    }
                } else {
                    // Off-beat notes are usually longer
                    if rng.random::<bool>() { // Changed to rng.random()
                        1.0
                    } else {
                        0.75
                    }
                };
                let actual_duration = beat_multiplier * seconds_per_quarter_note;
                
                if dur_sum + actual_duration > duration_seconds as f32 && !durations.is_empty() {
                    // Avoid adding a note that grossly exceeds total duration, unless it's the first note
                    break;
                }
                dur_sum += actual_duration;
                i += 1;
                durations.push(actual_duration); // Push actual duration in seconds
            }

            durations
        }
    };

    // Create note sequence
    let mut prev_note_idx = 0;
    let mut melody_notes: Vec<Note> = vec![];
    let total_beats: u32 = durations.len() as u32;
    for i in 0..total_beats {
        // For first note, start with the root note or fifth
        if i == 0 {
            let first_note_options = [0, 4]; // Root or fifth
            prev_note_idx = *first_note_options.choose(&mut rng).unwrap();
            let note = scale_notes[prev_note_idx].clone();
            let note_with_octave = Note::new(note.pitch_class, octave as u8);
            melody_notes.push(note_with_octave);
            continue;
        }

        // For natural progression, limit the jump size
        let mut possible_jumps = Vec::new();

        // Favor steps (1 or 2 indices away) over leaps
        for jump in [-2, -1, 1, 2].iter() {
            let new_idx = (prev_note_idx as i32 + jump) as usize;
            if new_idx < scale_notes.len() {
                // Add step moves multiple times to increase their probability
                possible_jumps.push(new_idx);
                possible_jumps.push(new_idx); // Duplicate to increase probability
            }
        }

        // Add occasional larger jumps for variety
        for jump in [-4, -3, 3, 4].iter() {
            let new_idx_signed = prev_note_idx as i32 + jump;
            if new_idx_signed >= 0 && new_idx_signed < scale_notes.len() as i32 {
                possible_jumps.push(new_idx_signed as usize);
            }
        }

        // For the last note, prefer ending on the root or fifth
        if i == total_beats - 1 {
            // Higher probability to end on root or fifth
            possible_jumps.extend(vec![0; 5]); // Root
            possible_jumps.push(4); // Fifth
        }

        // Choose the next note
        prev_note_idx = *possible_jumps.choose(&mut rng).unwrap_or(&0);
        let note = scale_notes[prev_note_idx].clone();

        // Determine octave (occasionally jump octaves for variety)
        let note_octave = if rng.random::<f32>() < 0.05 { // Changed to rng.random()
            // 10% chance to jump octave
            if rng.random::<bool>() { // Changed to rng.random()
                octave + 1
            } else {
                octave - 1
            }
        } else {
            octave
        };

        let note_with_octave = Note::new(note.pitch_class, note_octave as u8);
        melody_notes.push(note_with_octave);
    }

    // Generate the audio samples
    let mut all_samples = Vec::new();

    for (note, duration) in melody_notes.iter().zip(durations.iter()) {
        let frequency = note_to_frequency(note);
        let samples_for_note = (SAMPLE_RATE * duration) as usize;

        // Add a small gap between notes (articulation)
        let articulation = 1.0; // 85% of the note duration is played
        let sound_samples = (samples_for_note as f32 * articulation) as usize;
        let gap_samples = samples_for_note - sound_samples;

        // Generate the sine wave for this note
        let mut note_signal = dasp_signal::rate(SAMPLE_RATE as f64)
            .const_hz(frequency as f64)
            .square()
            .map(|x| (x * 0.5) as f32); // Half amplitude to prevent distortion

        // Add the sound part
        for _ in 0..sound_samples {
            all_samples.push(note_signal.next());
        }

        // Add the gap (silence) between notes
        all_samples.extend(vec![0.0; gap_samples]);
    }

    all_samples
}

/// Generate a melody that fits a specific chord progression style
pub fn get_melody(style: &str, root: u8, duration: u32, seconds_per_quarter_note: f32, seed: u64) -> Vec<f32> { // Changed bpm to seconds_per_quarter_note
    let root_pitch = semitone_to_pitch(root);
    let mut rng = StdRng::seed_from_u64(seed); // Changed from ChaCha8Rng. Initialize RNG here for consistent choices

    match style.to_lowercase().as_str() { // Added to_lowercase for consistency with gen.rs
        "blues" => {
            // Blues uses pentatonic minor scale typically
            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                Mode::Ionian,
                3,                         // Middle octave
                RhythmPattern::Syncopated, // Blues has syncopated rhythm
                duration,
                seconds_per_quarter_note, // Pass seconds_per_quarter_note
                seed,
            )
        }
        "pop" => {
            // Pop often uses major scale
            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                Mode::Ionian,          // Major scale
                3,                     // Middle octave
                RhythmPattern::Medium, // Pop usually has straightforward rhythm
                duration,
                seconds_per_quarter_note, // Pass seconds_per_quarter_note
                seed,
            )
        }
        "jazz" => {
            // Jazz often uses Dorian or Mixolydian scales
            let jazz_mode = if rng.random::<bool>() { // Use the seeded rng
                Mode::Dorian
            } else {
                Mode::Mixolydian
            };

            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                jazz_mode,
                3,                      // Middle octave
                RhythmPattern::Complex, // Jazz has complex rhythms
                duration,
                seconds_per_quarter_note, // Pass seconds_per_quarter_note
                seed,
            )
        }
        _ => {
            // Default to major scale
            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                Mode::Ionian, // Major scale
                3,            // Middle octave
                RhythmPattern::Simple,
                duration,
                seconds_per_quarter_note, // Pass seconds_per_quarter_note
                seed,
            )
        }
    }
}

/// Create a melody based on a specific scale with customizable parameters
pub fn create_custom_melody(
    root: u8,
    scale_type: &str,
    mode: &str,
    octave: i8,
    rhythm: &str,
    duration_total_seconds: f32, // Renamed for clarity
    seconds_per_quarter_note: f32, // Changed from bpm
    seed: u64,
) -> Vec<f32> {
    let root_pitch = semitone_to_pitch(root);

    // Parse scale type
    let scale_type = match scale_type.to_lowercase().as_str() {
        "diatonic" => ScaleType::Diatonic,
        "melodic_minor" => ScaleType::MelodicMinor,
        "harmonic_minor" => ScaleType::HarmonicMinor,
        _ => ScaleType::Diatonic, // Default to diatonic
    };

    // Parse mode
    let mode = match mode.to_lowercase().as_str() {
        "ionian" | "major" => Mode::Ionian,
        "dorian" => Mode::Dorian,
        "phrygian" => Mode::Phrygian,
        "lydian" => Mode::Lydian,
        "mixolydian" => Mode::Mixolydian,
        "aeolian" | "minor" => Mode::Aeolian,
        "locrian" => Mode::Locrian,
        _ => Mode::Ionian, // Default to major
    };

    // Parse rhythm pattern
    let rhythm_pattern = match rhythm.to_lowercase().as_str() {
        "simple" => RhythmPattern::Simple,
        "medium" => RhythmPattern::Medium,
        "complex" => RhythmPattern::Complex,
        "swung" => RhythmPattern::Swung,
        "syncopated" => RhythmPattern::Syncopated,
        _ => RhythmPattern::Simple, // Default to simple
    };

    generate_melody_samples(
        root_pitch,
        scale_type,
        mode,
        octave,
        rhythm_pattern,
        duration_total_seconds as u32, // Use the passed total duration
        seconds_per_quarter_note, // Pass seconds_per_quarter_note
        seed,
    )
}
