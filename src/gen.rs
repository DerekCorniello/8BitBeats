use crate::melodies;
use crate::progs;
use crate::tui::AppState;
use crossbeam_channel::{Receiver as CrossbeamReceiver, Sender as CrossbeamSender};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use std::thread;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 44100; // Audio sample rate in Hz

/* play_progression - Generates an audio sequence for a musical chord progression.
 *
 * Given a progression name (e.g., "blues", "pop"), a root note, and duration for each chord,
 * this function retrieves the chord voicings and their root notes. It then concatenates
 * the audio samples of these chords to form a continuous audio sequence.
 *
 * inputs:
 *     - prog_name (String): The name of the chord progression to use.
 *     - root_note (u8): The MIDI root note for the first chord of the progression.
 *     - chord_duration (f32): The duration in seconds for each chord in the progression.
 *
 * outputs:
 *     - (Vec<f32>, Vec<u8>): A tuple containing:
 *         - Vec<f32>: The concatenated audio samples of the chord progression.
 *         - Vec<u8>: A list of the root notes for each chord in the generated progression.
 */
fn play_progression(prog_name: String, root_note: u8, chord_duration: f32) -> (Vec<f32>, Vec<u8>) {
    let (progression_chords, progression_root_notes) =
        progs::get_progression(prog_name, root_note, chord_duration);

    // Combine all chord samples
    let mut audio_sequence = Vec::new();
    for chord in progression_chords {
        audio_sequence.extend_from_slice(&chord);
    }

    // Return the sequence, its duration, and the root notes
    (audio_sequence, progression_root_notes)
}

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
        let bass_note_midi = if chord_root >= 12 {
            chord_root - 12
        } else {
            chord_root
        };
        let bass_note_freq = note_to_freq(bass_note_midi);

        let time = (i % samples_per_chord) as f32 / SAMPLE_RATE as f32;
        let sample = (time * bass_note_freq * 2.0 * std::f32::consts::PI).sin();

        bass_line.push(sample * 0.6); // Bass notes are often attenuated or shaped differently.
    }

    bass_line
}

/* MusicControl - Defines commands to control the music playback service.
 *
 * These messages are sent from the TUI or other control points to the music generation
 * and playback thread to manage its state.
 */
pub enum MusicControl {
    Pause,     // Pauses current playback.
    Resume,    // Resumes current playback.
    Terminate, // Stops playback and terminates the music service thread.
    Rewind,    // Restarts the current song from the beginning.
}

/* MusicProgress - Reports the playback progress of the current song.
 *
 * Transmitted from the music service to the TUI to update progress indicators.
 *
 * fields:
 *     - current_samples (u64): Number of audio samples played so far.
 *     - total_samples (u64): Total number of audio samples in the current song.
 *     - actual_seed (u64): The seed value that was actually used to generate the current song.
 */
pub struct MusicProgress {
    pub current_samples: u64,
    pub total_samples: u64,
    pub actual_seed: u64,
}

/* MusicPlayer - Manages audio playback state and hardware interaction.
 *
 * This struct encapsulates the Rodio sink and stream, handles playback control messages,
 * and keeps track of the current audio data and playback position.
 *
 * fields:
 *     - receiver (CrossbeamReceiver<MusicControl>): Receives control messages.
 *     - sink (Sink): The Rodio audio sink for playing samples.
 *     - _stream (OutputStream): The Rodio output stream (held to keep audio active).
 *     - current_audio_data (Option<Vec<f32>>): Buffer for the currently loaded song's audio samples.
 *     - current_sample_rate (Option<u32>): Sample rate of the current audio data.
 *     - total_samples (u64): Total samples in `current_audio_data`.
 *     - playback_start_time (Option<Instant>): Timestamp of when playback last (re)started.
 *     - samples_played_at_pause (u64): Number of samples played before the last pause.
 *     - should_terminate (bool): Flag to signal the playback loop to exit.
 */
pub struct MusicPlayer {
    receiver: CrossbeamReceiver<MusicControl>,
    sink: Sink,
    _stream: OutputStream,
    current_audio_data: Option<Vec<f32>>,
    current_sample_rate: Option<u32>,
    total_samples: u64,
    playback_start_time: Option<Instant>,
    samples_played_at_pause: u64,
    should_terminate: bool,
}

impl MusicPlayer {
    /* new - Creates a new `MusicPlayer` instance.
     *
     * Initializes the audio output stream and sink, preparing for playback.
     * The sink starts in a paused state.
     *
     * inputs:
     *     - receiver (CrossbeamReceiver<MusicControl>): Channel to receive playback control messages.
     *
     * outputs:
     *     - Self: A new `MusicPlayer` instance.
     */
    pub fn new(receiver: CrossbeamReceiver<MusicControl>) -> Self {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get output stream");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");
        sink.pause();
        MusicPlayer {
            receiver,
            sink,
            _stream,
            current_audio_data: None,
            current_sample_rate: None,
            total_samples: 0,
            playback_start_time: None,
            samples_played_at_pause: 0,
            should_terminate: false,
        }
    }

    /* play_audio - Loads new audio data into the player and prepares it for playback.
     *
     * Stops any currently playing audio, replaces it with the new data, and resets
     * playback progress. The sink remains paused after loading; `Resume` is needed to start.
     *
     * inputs:
     *     - &mut self
     *     - audio_data (Vec<f32>): The raw audio samples to play.
     *     - sample_rate (u32): The sample rate of the provided `audio_data`.
     *
     * outputs:
     *     - None
     */
    pub fn play_audio(&mut self, audio_data: Vec<f32>, sample_rate: u32) {
        self.sink.stop();

        // Store the audio data and sample rate
        self.current_audio_data = Some(audio_data.clone());
        self.current_sample_rate = Some(sample_rate);

        let source = SamplesBuffer::new(1, sample_rate, audio_data);
        self.total_samples = self
            .current_audio_data
            .as_ref()
            .map_or(0, |d| d.len() as u64);
        self.samples_played_at_pause = 0;
        self.playback_start_time = None;

        self.sink.append(source);
    }

    /* should_continue - Checks if the music service should continue its playback loop.
     *
     * inputs:
     *     - &self
     *
     * outputs:
     *     - bool: True if the service should continue, false if termination is requested.
     */
    pub fn should_continue(&self) -> bool {
        !self.should_terminate
    }
}

/* generate_audio_from_state - Generates raw audio samples based on application state.
 *
 * This internal function takes the current `AppState` (scale, style, BPM, etc.) and
 * orchestrates calls to melody, chord progression, and bass line generation modules.
 * It then mixes these components and applies basic normalization.
 *
 * inputs:
 *     - app_state (&AppState): The current application state defining music parameters.
 *
 * outputs:
 *     - (Vec<f32>, u32, u64): A tuple containing:
 *         - Vec<f32>: The generated and mixed audio samples.
 *         - u32: The sample rate of the generated audio (typically `SAMPLE_RATE_AUDIO_GEN`).
 *         - u64: The actual seed value used for random number generation.
 */
fn generate_audio_from_state(app_state: &AppState) -> (Vec<f32>, u32, u64) {
    const SAMPLE_RATE_AUDIO_GEN: u32 = 44100;

    let root_note = match app_state.scale.to_owned().as_str() {
        "C" => 0,
        "C#" => 1,
        "D" => 2,
        "D#" => 3,
        "E" => 4,
        "F" => 5,
        "F#" => 6,
        "G" => 7,
        "G#" => 8,
        "A" => 9,
        "A#" => 10,
        "B" => 11,
        _ => 0, // Default to C
    };
    let duration_minutes = app_state
        .length
        .split_whitespace()
        .next()
        .unwrap_or("5")
        .parse::<f32>()
        .unwrap_or(5.0);
    let duration_seconds = duration_minutes * 60.0;
    let style = app_state.style.as_str();

    // Determine the actual seed to be used for generation
    let actual_generated_seed = app_state.seed.parse::<u64>().unwrap_or_else(|_| {
        // If seed string is empty or invalid, generate a truly random u64 seed value
        rand::random::<u64>()
    });
    let mut rng = StdRng::seed_from_u64(actual_generated_seed);

    let bpm_str = app_state.bpm.as_str();
    let bpm = match bpm_str.parse::<u32>() {
        Ok(val) if !bpm_str.is_empty() && val > 0 => val,
        _ => rng.gen_range(80..=160), // Corrected based on rand docs, will see if compiler still complains
    };

    let sec_per_beat: f32 = 60.0 / bpm as f32;
    let num_beats_per_chord = rng.gen_range(2..=4);
    let chord_duration: f32 = num_beats_per_chord as f32 * sec_per_beat;
    let samples_per_chord = (chord_duration * SAMPLE_RATE_AUDIO_GEN as f32) as usize;

    // Call get_melody and get_bass_line with their original signatures
    let melody = melodies::get_melody(
        style,
        root_note,
        duration_seconds as u32,
        sec_per_beat,
        actual_generated_seed,
    );
    let (chord_sequence, chord_root_notes) = match style.to_lowercase().as_str() {
        "blues" => play_progression(String::from("blues"), root_note, chord_duration),
        "pop" => play_progression(String::from("pop"), root_note, chord_duration),
        "jazz" => play_progression(String::from("jazz"), root_note, chord_duration),
        _ => play_progression(String::from("default"), root_note, chord_duration),
    };
    let melody_len = melody.len();
    let chord_len = chord_sequence.len();
    let target_len = melody_len;
    let bass_line = get_bass_line(
        style,
        &chord_root_notes,
        samples_per_chord,
        target_len,
        bpm,
        actual_generated_seed,
    );

    let mut mixed_audio = Vec::with_capacity(target_len);
    let chord_gain = 0.5;
    let melody_gain = 0.125;
    let bass_gain = 0.6;
    for i in 0..target_len {
        let chord_sample_val = if chord_len > 0 {
            chord_sequence.get(i % chord_len).copied().unwrap_or(0.0) * chord_gain
        } else {
            0.0
        };
        let melody_sample_val = melody.get(i).copied().unwrap_or(0.0) * melody_gain;
        let bass_sample_val = bass_line.get(i).copied().unwrap_or(0.0) * bass_gain;
        mixed_audio.push(melody_sample_val + chord_sample_val + bass_sample_val);
    }
    if !mixed_audio.is_empty() {
        let max_abs_val = mixed_audio
            .iter()
            .fold(0.0f32, |max, &val| max.max(val.abs()));
        if max_abs_val > 1.0 {
            for sample in &mut mixed_audio {
                *sample /= max_abs_val;
            }
        }
    }

    (mixed_audio, SAMPLE_RATE_AUDIO_GEN, actual_generated_seed)
}

/* run_music_service - Main function for the music generation and playback thread.
 *
 * This function initializes a `MusicPlayer`, generates initial audio based on `initial_app_state`,
 * and then enters a loop to handle control messages (Pause, Resume, Rewind, Terminate)
 * and report playback progress.
 *
 * inputs:
 *     - initial_app_state (AppState): The application state to use for generating the first song.
 *     - receiver (CrossbeamReceiver<MusicControl>): Channel to receive control messages.
 *     - progress_sender (CrossbeamSender<MusicProgress>): Channel to send progress updates.
 *
 * outputs:
 *     - None (runs in a separate thread until Terminate is received).
 */
pub fn run_music_service(
    initial_app_state: AppState,
    receiver: CrossbeamReceiver<MusicControl>,
    progress_sender: CrossbeamSender<MusicProgress>,
) {
    const SAMPLE_RATE_PROGRESS: f32 = 44100.0; // For progress calculation (as f32)

    thread::spawn(move || {
        let mut player = MusicPlayer::new(receiver);
        let current_app_state_for_generation = initial_app_state;
        let actual_seed_for_current_song: u64;

        // Initial audio generation based on initial_app_state
        {
            let (audio_data, sample_rate, seed) =
                generate_audio_from_state(&current_app_state_for_generation);
            actual_seed_for_current_song = seed;
            player.play_audio(audio_data, sample_rate); // Prepares sink, remains paused
            let _ = progress_sender.send(MusicProgress {
                // Send initial state
                current_samples: 0,
                total_samples: player.total_samples,
                actual_seed: actual_seed_for_current_song,
            });

            // Auto-start playback for the initial track
            if player.total_samples > 0 {
                // Ensure there's something to play
                player.playback_start_time = Some(Instant::now());
                player.sink.play();
            }
        }

        'service_loop: loop {
            // Process all pending control messages first
            loop {
                match player.receiver.try_recv() {
                    Ok(MusicControl::Pause) => {
                        if !player.sink.is_paused() && player.playback_start_time.is_some() {
                            let elapsed_since_last_play =
                                player.playback_start_time.unwrap().elapsed();
                            player.samples_played_at_pause +=
                                (elapsed_since_last_play.as_secs_f32() * SAMPLE_RATE_PROGRESS)
                                    as u64;
                            player.playback_start_time = None;
                        }
                        player.sink.pause();
                    }
                    Ok(MusicControl::Resume) => {
                        if player.sink.is_paused() && player.total_samples > 0 {
                            player.playback_start_time = Some(Instant::now());
                            player.sink.play();
                        }
                    }
                    Ok(MusicControl::Rewind) => {
                        if let (Some(audio_data_ref), Some(sample_rate_val)) =
                            (&player.current_audio_data, player.current_sample_rate)
                        {
                            // Clone the audio data to pass to play_audio
                            let audio_data_clone = audio_data_ref.clone();
                            player.play_audio(audio_data_clone, sample_rate_val);

                            // After play_audio, the sink is paused by default, so we need to resume it.
                            // Also, play_audio resets progress, so playback_start_time needs to be set here.
                            player.samples_played_at_pause = 0;
                            player.playback_start_time = Some(Instant::now()); // Set for immediate play
                            player.sink.play(); // Start playing from the beginning

                            let _ = progress_sender.send(MusicProgress {
                                current_samples: 0,
                                total_samples: player.total_samples,
                                actual_seed: actual_seed_for_current_song,
                            });
                        }
                    }
                    Ok(MusicControl::Terminate) => {
                        player.should_terminate = true;
                        player.sink.stop();
                        break 'service_loop;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        break; // No more messages, exit inner message loop
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        player.should_terminate = true;
                        break 'service_loop;
                    }
                }
            }

            if !player.should_continue() {
                break 'service_loop;
            }

            // Progress Reporting
            if player.total_samples > 0 && !player.should_terminate {
                let mut current_display_samples = player.samples_played_at_pause;
                if !player.sink.is_paused() && player.playback_start_time.is_some() {
                    let elapsed_since_current_play = player.playback_start_time.unwrap().elapsed();
                    current_display_samples +=
                        (elapsed_since_current_play.as_secs_f32() * SAMPLE_RATE_PROGRESS) as u64;
                }
                current_display_samples = current_display_samples.min(player.total_samples);

                let send_result = progress_sender.send(MusicProgress {
                    current_samples: current_display_samples,
                    total_samples: player.total_samples,
                    actual_seed: actual_seed_for_current_song,
                });
                if send_result.is_err() {
                    player.should_terminate = true;
                }

                if current_display_samples >= player.total_samples && !player.sink.is_paused() {
                    player.sink.pause();
                    player.playback_start_time = None;
                    player.samples_played_at_pause = player.total_samples;
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}

/* parse_song_id_to_app_state - Parses a song ID string into an `AppState`.
 *
 * The song ID format is expected to be "Scale-Style-BPM-Length-Seed", e.g., "C-Pop-120-5min-12345".
 * This function attempts to parse these components and construct an `AppState` suitable for
 * regenerating or loading the described song.
 *
 * inputs:
 *     - id_string (&str): The song ID string to parse.
 *
 * outputs:
 *     - Result<AppState, String>: Ok with the parsed `AppState` if successful,
 *                               or an Err with a descriptive message if parsing fails.
 */
pub fn parse_song_id_to_app_state(id_string: &str) -> Result<AppState, String> {
    let parts: Vec<&str> = id_string.split('-').collect();
    if parts.len() != 5 {
        return Err(format!(
            "Invalid Song ID: Expected 5 parts separated by '-'. Got {}. Format: Scale-Style-BPM-LengthInMinutes-Seed", 
            parts.len()
        ));
    }

    let scale = parts[0].to_string();
    let style = parts[1].to_string();
    let bpm_str = parts[2].to_string();
    let length_minutes_str = parts[3];
    let seed_str = parts[4].to_string();

    if bpm_str.parse::<u32>().is_err() && !bpm_str.is_empty() {
        return Err(format!(
            "Invalid BPM in Song ID: '{}' is not a valid number. Format: Scale-Style-BPM-LengthInMinutes-Seed", 
            bpm_str
        ));
    }

    let length_in_mins = match length_minutes_str.parse::<u32>() {
        Ok(mins) => format!("{} min", mins),
        Err(_) => {
            return Err(format!(
                "Invalid Length in Song ID: '{}' is not a valid number of minutes. Format: Scale-Style-BPM-LengthInMinutes-Seed", 
                length_minutes_str
            ));
        }
    };

    if seed_str.parse::<u64>().is_err() && !seed_str.is_empty() {
        return Err(format!(
           "Invalid Seed in Song ID: '{}' is not a valid number. Format: Scale-Style-BPM-LengthInMinutes-Seed", 
           seed_str
        ));
    }

    Ok(AppState {
        scale,
        style,
        bpm: bpm_str,
        length: length_in_mins,
        seed: seed_str,
        ..Default::default()
    })
}
