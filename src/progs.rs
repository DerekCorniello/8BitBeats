use rust_music_theory::note::{Note, Notes, PitchClass};

use dasp_signal::Signal;
use rust_music_theory::chord::{Chord, Number as ChordNumber, Quality as ChordQuality};

/// Extension trait to add numeric conversion methods to PitchClass
trait PitchClassExt {
    /// Convert to semitone offset (0-11)
    fn to_semitone(&self) -> i32;

    /// Create from numeric value
    fn from_numeric(value: u8) -> Self;
}

impl PitchClassExt for PitchClass {
    fn to_semitone(&self) -> i32 {
        // Convert PitchClass to its semitone value
        match self {
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

    fn from_numeric(value: u8) -> Self {
        // Create PitchClass from numeric value
        match value % 12 {
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
            _ => unreachable!(), // Unreachable due to modulo 12
        }
    }
}

/// Convert a Note to its MIDI number.
/// MIDI numbers represent notes in a standardized way where each number is a specific pitch.
fn note_to_midi(note: &Note) -> i32 {
    // Get the semitone offset based on the pitch class
    let semitone = note.pitch_class.to_semitone();

    // Calculate MIDI number based on octave and semitone
    // Formula: (octave+1) * 12 + semitone
    (note.octave as i32 + 1) * 12 + semitone
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
pub fn generate_chord_samples(
    root_note: PitchClass,       // The root note of the chord (C, D, etc.)
    chord_quality: ChordQuality, // Major, minor, diminished, etc.
    chord_type: ChordNumber,     // Triad, seventh, ninth, etc.
    duration_seconds: f32,       // How long the chord should play
    sample_rate: u32,            // Audio quality (samples per second)
) -> Vec<f32> {
    // Create a chord object using the music theory library
    let chord = Chord::new(root_note, chord_quality, chord_type);

    // Get the actual notes in the chord
    let chord_notes = chord.notes();

    // Calculate the frequency for each note in the chord
    let note_frequencies: Vec<f32> = chord_notes.iter().map(note_to_frequency).collect();

    // Generate sine wave signals for each frequency
    let mut note_generators: Vec<_> = note_frequencies
        .iter()
        .map(|&freq| {
            dasp_signal::rate(sample_rate as f64)
                .const_hz(freq as f64)
                .sine()
                .map(|x| (x * 0.4) as f32) // Increased initial amplitude to 0.4
        })
        .collect();

    // Calculate the total number of samples needed
    let total_samples = (sample_rate as f32 * duration_seconds) as usize;
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

/// Get a PitchClass from a numeric value
pub fn get_pitch(root: u8) -> PitchClass {
    PitchClass::from_numeric(root)
}

/// Get a chord progression by name
pub fn get_progression(prog_name: String, root: u8, chord_duration: f32) -> (Vec<Vec<f32>>, Vec<u8>) {
    let sample_rate = 44100; // Standard CD-quality audio
    let mut chord_samples_list = Vec::new();
    let mut root_notes_list = Vec::new();

    // Define a helper closure to generate chord and collect root note
    let mut add_chord = |current_root_offset: u8, quality: ChordQuality, number: ChordNumber| {
        let absolute_root = root + current_root_offset;
        // Convert to MIDI note: C4 (MIDI 60) is a common middle C.
        // Our `root` (0-11) + `absolute_root` (relative to root)
        // To make it concrete, let's assume the `root` from UI corresponds to an octave (e.g. octave 3 or 4).
        // The `PitchClass::from_numeric(absolute_root)` handles wrapping around 12.
        // The `chord.notes()` then uses an octave (defaulting to 4 if not specified or derived).
        // Let's ensure our `absolute_root` for bass is a MIDI note number.
        // The `Note` struct in `rust-music-theory` uses octave numbers. C4 is `PitchClass::C` at `octave: 4`.
        // `note_to_midi` converts `Note` to MIDI. `(note.octave as i32 + 1) * 12 + semitone`
        // If `PitchClass::C` (semitone 0) is at octave 4, MIDI is (4+1)*12 + 0 = 60.
        // So `get_pitch(absolute_root)` is fine for `generate_chord_samples` as it expects `PitchClass`.
        // For the bass line, we need a consistent MIDI note. Let's use octave 3 for chord roots.
        // The `root` (0-11) from UI + `current_root_offset`. Bass will be octave 2.
        let chord_root_midi = root + current_root_offset + 12 * 3; // Assuming octave 3 for chord root
        root_notes_list.push(chord_root_midi);
        chord_samples_list.push(generate_chord_samples(
            get_pitch(absolute_root), // This is fine, uses the 0-11 pitch class
            quality,
            number,
            chord_duration,
            sample_rate,
        ));
    };

    match prog_name.to_lowercase().as_str() {
        "blues" => {
            add_chord(0, ChordQuality::Major, ChordNumber::Triad);    // I
            add_chord(5, ChordQuality::Major, ChordNumber::Triad);    // IV
            add_chord(7, ChordQuality::Major, ChordNumber::Triad);    // V
            add_chord(5, ChordQuality::Major, ChordNumber::Triad);    // IV
        }
        "pop" => {
            add_chord(0, ChordQuality::Major, ChordNumber::Triad);    // I
            add_chord(7, ChordQuality::Major, ChordNumber::Triad);    // V
            add_chord(9, ChordQuality::Minor, ChordNumber::Triad);    // vi
            add_chord(5, ChordQuality::Major, ChordNumber::Triad);    // IV
        }
        "jazz" => {
            add_chord(2, ChordQuality::Minor, ChordNumber::Seventh);  // ii
            add_chord(7, ChordQuality::Dominant, ChordNumber::Seventh);// V
            add_chord(0, ChordQuality::Major, ChordNumber::Seventh);  // I
        }
        _ => { // Default to a simple I-IV progression
            add_chord(0, ChordQuality::Major, ChordNumber::Triad);    // I
            add_chord(5, ChordQuality::Major, ChordNumber::Triad);    // IV
        }
    }
    (chord_samples_list, root_notes_list)
}
