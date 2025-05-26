use dasp_signal::Signal;
use rand::prelude::*;
use rand::rngs::StdRng;
use rust_music_theory::note::{Note, Notes, PitchClass};
use rust_music_theory::scale::{Direction, Mode, Scale, ScaleType};

/* pitch_to_semitone - Converts a `PitchClass` to its semitone offset from C.
 *
 * (C=0, C#=1, ..., B=11)
 *
 * inputs:
 *     - pitch (&PitchClass): The pitch class to convert.
 *
 * outputs:
 *     - u8: The semitone offset (0-11).
 */
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

/* semitone_to_pitch - Converts a semitone offset (from C) back to a `PitchClass`.
 *
 * Wraps around 12, so 12 becomes C, 13 becomes C#, etc.
 *
 * inputs:
 *     - semitone (u8): The semitone offset (0-11 typically, but handles larger values).
 *
 * outputs:
 *     - PitchClass: The corresponding pitch class.
 */
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

/* note_to_frequency - Converts a `Note` (pitch class and octave) to its frequency in Hz.
 *
 * Uses the standard A4=440Hz tuning reference.
 *
 * inputs:
 *     - note (&Note): The note to convert.
 *
 * outputs:
 *     - f32: The frequency of the note in Hertz.
 */
fn note_to_frequency(note: &Note) -> f32 {
    let octave_offset = (note.octave as i32 + 1) * 12;
    let semitone = pitch_to_semitone(&note.pitch_class) as i32;
    let midi_number = octave_offset + semitone;

    // Standard formula: A4 (MIDI 69) = a440, each semitone is 2^(1/12)
    440.0 * 2f32.powf((midi_number as f32 - 69.0) / 12.0)
}

/* RhythmPattern - Defines different rhythmic feels for melody generation.
 *
 * Each variant implies a different distribution of note durations.
 */
pub enum RhythmPattern {
    Simple,     // Primarily quarter notes (1 note per beat).
    Medium,     // Mix of quarter and eighth notes (1-2 notes per beat).
    Complex,    // Mix of eighth and sixteenth notes, allowing for faster passages.
    Syncopated, // Emphasizes off-beat notes for a syncopated feel.
}

/* generate_melody_samples - Generates a sequence of audio samples for a melody.
 *
 * This function constructs a melody based on musical scale, rhythm, and duration.
 * It involves several steps:
 * 1. Defining note durations based on the `rhythm_pattern`.
 * 2. Selecting a sequence of notes from the specified `scale` with probabilistic transitions.
 * 3. Synthesizing audio samples for each note using a simple sine wave and an ADSR envelope.
 * 4. Applying articulation (small gaps) between notes.
 *
 * inputs:
 *     - root_note (PitchClass): The tonic of the scale for the melody.
 *     - scale_type (ScaleType): The type of scale (e.g., Major, Minor).
 *     - mode (Mode): The mode of the scale (e.g., Ionian, Dorian).
 *     - octave (i8): The base octave for the melody notes.
 *     - rhythm_pattern (RhythmPattern): The rhythmic feel to apply.
 *     - duration_seconds (u32): Total desired duration of the melody in seconds.
 *     - seconds_per_quarter_note (f32): Duration of a single quarter note, derived from BPM.
 *     - seed (u64): Seed for the random number generator to ensure reproducibility.
 *
 * outputs:
 *     - Vec<f32>: A vector of f32 audio samples representing the generated melody at SAMPLE_RATE.
 */
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
                let actual_duration = if rng.gen::<bool>() { // CORRECTED
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
                let roll = rng.gen::<f32>(); // CORRECTED
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
        RhythmPattern::Syncopated => {
            // Syncopated rhythm with some off-beat notes
            // let mut durations = vec![]; // durations is already mutably borrowed

            // counts if beat is on beat or off beat
            let mut i = 0;
            dur_sum = 0.0; // Reset dur_sum for this pattern
            while dur_sum < duration_seconds as f32 {
                let beat_multiplier = if i % 2 == 0 {
                    // On-beat notes are usually shorter
                    if rng.gen::<bool>() { // CORRECTED
                        0.5
                    } else {
                        0.25
                    }
                } else {
                    // Off-beat notes are usually longer
                    if rng.gen::<bool>() { // CORRECTED
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
        let note_octave = if rng.gen::<f32>() < 0.05 { // CORRECTED
            // 10% chance to jump octave, corrected to 5%
            if rng.gen::<bool>() { // CORRECTED
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

/* get_melody - Generates melody audio samples based on style, root note, and duration.
 *
 * This function acts as a high-level selector for melody generation. It interprets the
 * `style` string to choose appropriate scale, mode, rhythm, and octave parameters,
 * then calls `generate_melody_samples` to create the audio.
 *
 * inputs:
 *     - style (&str): Musical style string (e.g., "pop", "rock", "jazz", "blues").
 *     - root (u8): MIDI root note of the scale (0-11).
 *     - duration (u32): Total desired duration of the melody in seconds.
 *     - seconds_per_quarter_note (f32): Duration of a single quarter note, derived from BPM.
 *     - seed (u64): Seed for random number generation.
 *
 * outputs:
 *     - Vec<f32>: A vector of f32 audio samples representing the generated melody.
 */
pub fn get_melody(style: &str, root: u8, duration: u32, seconds_per_quarter_note: f32, seed: u64) -> Vec<f32> {
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
            let jazz_mode = if rng.gen::<bool>() { // Use the seeded rng
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
