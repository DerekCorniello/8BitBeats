use dasp_signal::Signal;
use rand::{seq::SliceRandom, Rng};
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
}

/// Generate melody samples based on given parameters
pub fn generate_melody_samples(
    root_note: PitchClass,
    scale_type: ScaleType,
    mode: Mode,
    octave: i8,
    num_notes: usize,
    rhythm_pattern: RhythmPattern,
    duration_seconds: f32,
    sample_rate: u32,
) -> Vec<f32> {
    let mut rng = rand::rng();

    // Create scale
    let scale = Scale::new(
        ScaleType::Diatonic, // scale type
        PitchClass::C,       // tonic
        4,                   // octave
        Some(Mode::Ionian),  // scale mode
        Direction::Ascending,
    )
    .unwrap();

    let scale_notes = scale.notes();

    println!(
        "Generating melody in {:?} {:?} {:?}",
        root_note, scale_type, mode
    );
    println!("Scale notes: {:?}", scale_notes);

    // Create note sequence
    let mut melody_notes = Vec::with_capacity(num_notes);
    let mut prev_note_idx = 0;

    for i in 0..num_notes {
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
        if i == num_notes - 1 {
            // Higher probability to end on root or fifth
            for _ in 0..5 {
                possible_jumps.push(0); // Root
            }
            possible_jumps.push(4); // Fifth
        }

        // Choose the next note
        prev_note_idx = *possible_jumps.choose(&mut rng).unwrap_or(&0);
        let note = scale_notes[prev_note_idx].clone();

        // Determine octave (occasionally jump octaves for variety)
        let note_octave = if rng.random::<f32>() < 0.1 {
            // 10% chance to jump octave
            if rng.random::<bool>() {
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

    // Apply rhythm pattern
    let (durations, total_beats) = match rhythm_pattern {
        RhythmPattern::Simple => {
            // All quarter notes
            let durations = vec![1.0; num_notes];
            (durations, num_notes as f32)
        }
        RhythmPattern::Medium => {
            // Mix of quarter and eighth notes
            let mut durations = Vec::with_capacity(num_notes);
            let mut beats_used = 0.0;

            for _ in 0..num_notes {
                // 50% chance of quarter note, 50% chance of eighth note
                let duration = if rng.random::<bool>() { 1.0 } else { 0.5 };
                durations.push(duration);
                beats_used += duration;
            }

            (durations, beats_used)
        }
        RhythmPattern::Complex => {
            // Mix of quarter, eighth, and sixteenth notes
            let mut durations = Vec::with_capacity(num_notes);
            let mut beats_used = 0.0;

            for _ in 0..num_notes {
                // 25% quarter, 50% eighth, 25% sixteenth
                let roll = rng.random::<f32>();
                let duration = if roll < 0.25 {
                    1.0 // quarter
                } else if roll < 0.75 {
                    0.5 // eighth
                } else {
                    0.25 // sixteenth
                };

                durations.push(duration);
                beats_used += duration;
            }

            (durations, beats_used)
        }
        RhythmPattern::Syncopated => {
            // Syncopated rhythm with some off-beat notes
            let mut durations = Vec::with_capacity(num_notes);
            let mut beats_used = 0.0;

            for i in 0..num_notes {
                let duration = if i % 2 == 0 {
                    // On-beat notes are usually shorter
                    if rng.random::<bool>() {
                        0.5
                    } else {
                        0.25
                    }
                } else {
                    // Off-beat notes are usually longer
                    if rng.random::<bool>() {
                        1.0
                    } else {
                        0.75
                    }
                };

                durations.push(duration);
                beats_used += duration;
            }

            (durations, beats_used)
        }
    };

    // Calculate time for each beat
    let beat_time = duration_seconds / total_beats;

    // Generate the audio samples
    let mut all_samples = Vec::new();

    for (note, duration) in melody_notes.iter().zip(durations.iter()) {
        let frequency = note_to_frequency(note);
        let note_duration = beat_time * duration;
        let samples_for_note = (sample_rate as f32 * note_duration) as usize;

        // Add a small gap between notes (articulation)
        let articulation = 0.85; // 85% of the note duration is played
        let sound_samples = (samples_for_note as f32 * articulation) as usize;
        let gap_samples = samples_for_note - sound_samples;

        // Generate the sine wave for this note
        let mut note_signal = dasp_signal::rate(sample_rate as f64)
            .const_hz(frequency as f64)
            .sine()
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
pub fn get_melody(style: &str, root: u8, duration: f32, num_notes: usize) -> Vec<f32> {
    let sample_rate = 44100; // Standard CD-quality audio
    let root_pitch = semitone_to_pitch(root);

    match style {
        "blues" => {
            println!("Creating blues melody in {:?}", root_pitch);
            // Blues uses pentatonic minor scale typically
            generate_melody_samples(
                root_pitch,
                ScaleType::Major,
                Mode::Minor,
                4, // Middle octave
                num_notes,
                RhythmPattern::Syncopated, // Blues has syncopated rhythm
                duration,
                sample_rate,
            )
        }
        "pop" => {
            println!("Creating pop melody in {:?}", root_pitch);
            // Pop often uses major scale
            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                Mode::Ionian, // Major scale
                4,            // Middle octave
                num_notes,
                RhythmPattern::Medium, // Pop usually has straightforward rhythm
                duration,
                sample_rate,
            )
        }
        "jazz" => {
            println!("Creating jazz melody in {:?}", root_pitch);
            // Jazz often uses Dorian or Mixolydian scales
            let jazz_mode = if rand::thread_rng().random::<bool>() {
                Mode::Dorian
            } else {
                Mode::Mixolydian
            };

            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                jazz_mode,
                4, // Middle octave
                num_notes,
                RhythmPattern::Complex, // Jazz has complex rhythms
                duration,
                sample_rate,
            )
        }
        _ => {
            println!("Creating default melody in {:?}", root_pitch);
            // Default to major scale
            generate_melody_samples(
                root_pitch,
                ScaleType::Diatonic,
                Mode::Ionian, // Major scale
                4,            // Middle octave
                num_notes,
                RhythmPattern::Simple,
                duration,
                sample_rate,
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
    num_notes: usize,
    rhythm: &str,
    duration: f32,
) -> Vec<f32> {
    let sample_rate = 44100;
    let root_pitch = semitone_to_pitch(root);

    // Parse scale type
    let scale_type = match scale_type.to_lowercase().as_str() {
        "diatonic" => ScaleType::Diatonic,
        "melodic_minor" => ScaleType::MelodicMinor,
        "harmonic_minor" => ScaleType::HarmonicMinor,
        "harmonic_major" => ScaleType::HarmonicMajor,
        "pentatonic" => ScaleType::Pentatonic,
        "blues" => ScaleType::Blues,
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
        "syncopated" => RhythmPattern::Syncopated,
        _ => RhythmPattern::Simple, // Default to simple
    };

    generate_melody_samples(
        root_pitch,
        scale_type,
        mode,
        octave,
        num_notes,
        rhythm_pattern,
        duration,
        sample_rate,
    )
}
