const SAMPLE_RATE: u32 = 44100; // Audio sample rate in Hz

/* note_to_freq - Converts a MIDI-like note number to its corresponding frequency in Hertz.
 *
 * This function uses the standard A4 = 440 Hz tuning convention, where A4 corresponds to MIDI note 57 (0-indexed)
 * or 69 (1-indexed). The formula implemented is: frequency = 440 * 2^((note - 57) / 12).
 * It assumes a 0-indexed MIDI note system where C0 is 0, C4 (middle C) is 48.
 *
 * inputs:
 *     - note (u8): The MIDI-like note number (0-indexed, e.g., C4 = 48, A4 = 57).
 *
 * outputs:
 *     - f32: The frequency of the note in Hz.
 */
fn note_to_freq(note: u8) -> f32 {
    440.0 * (2.0f32).powf((note as f32 - 57.0) / 12.0) // MIDI A4 = 57 (0-indexed)
}

/* get_bass_line - Generates a simple bass line based on a chord progression.
 *
 * The bass line plays the root note of each chord, transposed one octave lower.
 * The input `chord_root_notes` are expected to be absolute MIDI-like note numbers.
 * For example, if a chord root is C4 (MIDI 60), the bass will play C3 (MIDI 48).
 * If transposing a note down an octave would result in a MIDI note number less than 0,
 * the original note is used (this effectively means notes below C1 will not be transposed further down).
 * The output is a sequence of raw audio samples representing a sine wave for each bass note.
 *
 * inputs:
 *     - _style (&str): Style of the bass line (currently unused, for future variations).
 *     - chord_root_notes (&Vec<u8>): A vector of MIDI-like note numbers representing the root of each chord in the progression cycle.
 *     - samples_per_chord (usize): The number of audio samples each bass note (corresponding to a chord) should last.
 *     - total_samples (usize): The total desired length of the bass line in audio samples, typically to match a melody.
 *     - _bpm (u32): Beats per minute (currently unused, for future rhythmic variations).
 *     - _seed (u64): Seed for randomization (currently unused, for future randomization).
 *
 * outputs:
 *     - Vec<f32>: A vector of f32 audio samples representing the generated bass line.
 */
pub fn get_bass_line(
    _style: &str,
    chord_root_notes: &[u8],
    samples_per_chord: usize,
    total_samples: usize,
    _bpm: u32,
    _seed: u64,
) -> Vec<f32> {
    if chord_root_notes.is_empty() || samples_per_chord == 0 {
        return vec![0.0; total_samples];
    }

    let mut bass_line = Vec::with_capacity(total_samples);
    let num_chords_in_progression = chord_root_notes.len();

    for i in 0..total_samples {
        let current_chord_index = (i / samples_per_chord) % num_chords_in_progression;
        let chord_root = chord_root_notes[current_chord_index];

        // Play bass note one octave lower than the chord root.
        // If chord_root is C1 (MIDI 12) or higher, subtract 12. Otherwise, use chord_root.
        let bass_note_midi = if chord_root >= 12 { chord_root - 12 } else { chord_root };
        let bass_note_freq = note_to_freq(bass_note_midi);

        let time = (i % samples_per_chord) as f32 / SAMPLE_RATE as f32;
        let sample = (time * bass_note_freq * 2.0 * std::f32::consts::PI).sin();
        
        bass_line.push(sample * 0.6); // Bass notes are often attenuated or shaped differently.
    }

    bass_line
}
