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
            dasp_signal::rate(sample_rate as f64) // Set the sample rate
                .const_hz(freq as f64) // Create a constant frequency
                .square() // Generate a sine wave
                .map(|x| (x * 0.1) as f32) // Reduce amplitude to avoid distortion
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
pub fn mix_samples(sample_collections: Vec<Vec<f32>>, volume_levels: &[f32]) -> Vec<f32> {
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

/// Get a PitchClass from a numeric value
pub fn get_pitch(root: u8) -> PitchClass {
    PitchClass::from_numeric(root)
}

/// Get a chord progression by name
pub fn get_progression(prog_name: String, root: u8, chord_duration: f32) -> Vec<Vec<f32>> {
    let sample_rate = 44100; // Standard CD-quality audio

    match prog_name.as_str() {
        "blues" => {
            println!("Creating blues progression in {:?}", get_pitch(root));
            vec![
                // I chord
                generate_chord_samples(
                    get_pitch(root),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // IV chord
                generate_chord_samples(
                    get_pitch(root + 5), // Perfect fourth up from root
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // V chord
                generate_chord_samples(
                    get_pitch(root + 7), // Perfect fifth up from root
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // IV chord again
                generate_chord_samples(
                    get_pitch(root + 5),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
            ]
        }
        "pop" => {
            println!("Creating pop progression in {:?}", get_pitch(root));
            vec![
                // I chord
                generate_chord_samples(
                    get_pitch(root),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // V chord
                generate_chord_samples(
                    get_pitch(root + 7),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // vi chord
                generate_chord_samples(
                    get_pitch(root + 9),
                    ChordQuality::Minor,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                // IV chord
                generate_chord_samples(
                    get_pitch(root + 5),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
            ]
        }
        "jazz" => {
            println!("Creating jazz ii-V-I progression in {:?}", get_pitch(root));
            vec![
                // ii chord (minor 7th)
                generate_chord_samples(
                    get_pitch(root + 2),
                    ChordQuality::Minor,
                    ChordNumber::Seventh,
                    chord_duration,
                    sample_rate,
                ),
                // V chord (dominant 7th)
                generate_chord_samples(
                    get_pitch(root + 7),
                    ChordQuality::Dominant,
                    ChordNumber::Seventh,
                    chord_duration,
                    sample_rate,
                ),
                // I chord (major 7th)
                generate_chord_samples(
                    get_pitch(root),
                    ChordQuality::Major,
                    ChordNumber::Seventh,
                    chord_duration,
                    sample_rate,
                ),
            ]
        }
        // Default to a simple I-IV progression if the name doesn't match
        _ => {
            println!("Using default I-IV progression in {:?}", get_pitch(root));
            vec![
                generate_chord_samples(
                    get_pitch(root),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
                generate_chord_samples(
                    get_pitch(root + 5),
                    ChordQuality::Major,
                    ChordNumber::Triad,
                    chord_duration,
                    sample_rate,
                ),
            ]
        }
    }
}
