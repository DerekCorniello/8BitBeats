use crate::melodies;
use crate::progs;

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink, Source};

fn play_progression(prog_name: String, root_note: u8, chord_duration: f32) -> Vec<f32> {
    // Get the progression chords
    let progression = progs::get_progression(prog_name, root_note, chord_duration);

    // Combine all chord samples
    let mut audio_sequence = Vec::new();
    for chord in progression {
        audio_sequence.extend_from_slice(&chord);
    }

    // Return the sequence and its duration
    audio_sequence
}

pub fn play_music() {
    const SAMPLE_RATE: u32 = 44100; // CD-quality audio (44.1 kHz)
    let root_note = 0;
    let bpm = 120.0;
    let sec_per_beat = 60.0 / bpm;
    let chord_duration = 4.0 * sec_per_beat;
    let duration = 60.0;
    let style = "blues";
    let seed = 1;

    let melody = melodies::get_melody(style, root_note, duration as u32, bpm as u32, seed);

    let chord_sequence = match style {
        "blues" => play_progression(String::from("blues"), root_note, chord_duration),
        "pop" => play_progression(String::from("pop"), root_note, chord_duration),
        "jazz" => play_progression(String::from("jazz"), root_note, chord_duration),
        _ => play_progression(String::from("default"), root_note, chord_duration),
    };

    let mut mixed_audio = Vec::with_capacity(chord_sequence.len().max(melody.len()));
    let chord_gain = 0.4; // Slightly lower volume for chords
    let melody_gain = 0.6; // Slightly higher volume for melody

    // Add samples together with appropriate gain to avoid clipping
    for i in 0..mixed_audio.capacity() {
        let chord_sample = if i < chord_sequence.len() {
            chord_sequence[i] * chord_gain
        } else {
            0.0
        };
        let melody_sample = if i < melody.len() {
            melody[i] * melody_gain
        } else {
            0.0
        };
        mixed_audio.push(chord_sample + melody_sample);
    }

    // Set up audio output system
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to get output stream");
    // Create a new audio sink (output)
    let audio_sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

    // Prepare mixed audio data and set it to repeat
    let audio_source = SamplesBuffer::new(2, SAMPLE_RATE, mixed_audio.clone()).repeat_infinite();

    // Add the audio to the sink
    audio_sink.append(audio_source);

    // Continue until the sink is empty or we're told to stop
    while !audio_sink.empty() {
        // Check if playback should be paused
        audio_sink.play();
    }
}
