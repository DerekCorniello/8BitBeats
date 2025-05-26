// use rand::{Rng, SeedableRng}; // Removed unused imports for now
// use rand_pcg::Pcg64; // Removed unused import

const SAMPLE_RATE: u32 = 44100;

// Basic note to frequency (A4 = 440 Hz, note 69)
// MIDI note numbers: C0=0, C#0=1, ..., A4=57+12=69, ... C8=96+12=108
// Adjusted to use C4 as note 60 for easier octave calculations relative to middle C.
// Our root_note is 0-11 for C to B.
// So, C in octave 4 would be root_note + 12 * 4.
// Let's assume note 0 = C0. So C4 = 12 * 4 = 48.
fn note_to_freq(note: u8) -> f32 {
    440.0 * (2.0f32).powf((note as f32 - 57.0) / 12.0) // MIDI A4 = 57 (0-indexed)
}


pub fn get_bass_line(
    _style: &str, // For future variations
    chord_root_notes: &Vec<u8>, // Root notes of the chord progression (one cycle)
    samples_per_chord: usize,   // How many audio samples each chord/bass note lasts
    total_samples: usize,       // Desired total length of the bass line (to match melody)
    _bpm: u32, // For future rhythmic variations
    _seed: u64, // For future randomization
) -> Vec<f32> {
    if chord_root_notes.is_empty() || samples_per_chord == 0 {
        return vec![0.0; total_samples];
    }

    let mut bass_line = Vec::with_capacity(total_samples);
    let num_chords_in_progression = chord_root_notes.len();

    for i in 0..total_samples {
        let current_chord_index = (i / samples_per_chord) % num_chords_in_progression;
        let chord_root = chord_root_notes[current_chord_index];

        // Play bass note one octave lower than the chord root
        // Assuming chord_root is 0-11 for C-B in some octave.
        // To go one octave lower, subtract 12.
        // Make sure it doesn't go below a minimum (e.g., C2, which is MIDI note 36, our note 24 if C0=0)
        // Let's assume our input `chord_root_notes` are relative to C in *some* octave (e.g. root_note from AppState)
        // and are 0-11. So if root_note from AppState is C4 (MIDI 60), then a chord root of 0 is C4.
        // A bass note an octave down would be C3 (MIDI 48).
        // For simplicity, let's say the passed `chord_root_notes` are already absolute MIDI-like numbers.
        // The `progs.rs` `get_progression` takes a `root_note` (0-11) and `chord_duration`.
        // It calculates notes like `root_note + prog_step + 12 * 4` (for octave 4).
        // So the `chord_root_notes` we will receive will be absolute MIDI-like values.
        let bass_note_midi = if chord_root >= 12 { chord_root - 12 } else { chord_root }; // Octave down
        let bass_note_freq = note_to_freq(bass_note_midi);

        let time = (i % samples_per_chord) as f32 / SAMPLE_RATE as f32;
        let sample = (time * bass_note_freq * 2.0 * std::f32::consts::PI).sin();
        
        bass_line.push(sample * 0.7); // Bass notes often a bit quieter or shaped differently
    }

    bass_line
}

// Example usage (for testing, can be removed)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bass_line() {
        // Cmaj (MIDI 60), Gmaj (MIDI 67)
        let chord_roots = vec![60u8, 67u8]; 
        let samples_per_chord = SAMPLE_RATE as usize; // 1 second per chord
        let total_samples = 2 * samples_per_chord; // 2 seconds total
        let bpm = 120;
        let seed = 0;

        let bass = get_bass_line("default", &chord_roots, samples_per_chord, total_samples, bpm, seed);
        assert_eq!(bass.len(), total_samples);
        // Basic check: not all zeros
        assert!(bass.iter().any(|&s| s.abs() > 0.001));

        // Check frequencies (approx)
        // Bass C3 (MIDI 48), G3 (MIDI 55)
        // Freq C3 ~ 130.81 Hz, G3 ~ 196.00 Hz
        // Samples for C3 should have ~130 cycles in 1s.
        // Samples for G3 should have ~196 cycles in 1s.
    }
} 