use crate::bass;
use crate::melodies;
use crate::progs;
use crate::tui::AppState;
use crossbeam_channel::{Receiver as CrossbeamReceiver, Sender as CrossbeamSender};
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng
};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use std::thread;
use std::time::{Duration, Instant};

fn play_progression(prog_name: String, root_note: u8, chord_duration: f32) -> (Vec<f32>, Vec<u8>) {
    // Get the progression chords and their root notes
    let (progression_chords, progression_root_notes) = progs::get_progression(prog_name, root_note, chord_duration);

    // Combine all chord samples
    let mut audio_sequence = Vec::new();
    for chord in progression_chords {
        audio_sequence.extend_from_slice(&chord);
    }

    // Return the sequence, its duration, and the root notes
    (audio_sequence, progression_root_notes)
}

pub enum MusicControl {
    Pause,
    Resume,
    Terminate,
    Rewind,
}

// New struct for progress updates
pub struct MusicProgress {
    pub current_samples: u64,
    pub total_samples: u64,
    pub actual_seed: u64,
}

pub struct MusicPlayer {
    receiver: CrossbeamReceiver<MusicControl>,
    sink: Sink,
    _stream: OutputStream,
    current_audio_data: Option<Vec<f32>>, // Added to store current audio data
    current_sample_rate: Option<u32>,    // Added to store current sample rate
    total_samples: u64,
    playback_start_time: Option<Instant>,
    samples_played_at_pause: u64,
    should_terminate: bool,
}

impl MusicPlayer {
    pub fn new(receiver: CrossbeamReceiver<MusicControl>) -> Self {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get output stream");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");
        sink.pause();
        MusicPlayer {
            receiver,
            sink,
            _stream,
            current_audio_data: None, // Initialize
            current_sample_rate: None, // Initialize
            total_samples: 0,
            playback_start_time: None,
            samples_played_at_pause: 0,
            should_terminate: false,
        }
    }

    pub fn play_audio(&mut self, audio_data: Vec<f32>, sample_rate: u32) {
        self.sink.stop(); 

        // Store the audio data and sample rate
        self.current_audio_data = Some(audio_data.clone());
        self.current_sample_rate = Some(sample_rate);

        let source = SamplesBuffer::new(1, sample_rate, audio_data);
        self.total_samples = self.current_audio_data.as_ref().map_or(0, |d| d.len() as u64);
        self.samples_played_at_pause = 0;
        self.playback_start_time = None; 
        
        self.sink.append(source);
    }

    pub fn should_continue(&self) -> bool {
        !self.should_terminate
    }
}

// Internal function to generate audio based on AppState
// Returns (audio_data, sample_rate, actual_seed_used)
fn generate_audio_from_state(app_state: &AppState) -> (Vec<f32>, u32, u64) { 
    const SAMPLE_RATE_AUDIO_GEN: u32 = 44100;

    let root_note = match app_state.scale.to_owned().as_str() {
        "C" => 0, "C#" => 1, "D" => 2, "D#" => 3, "E" => 4, "F" => 5,
        "F#" => 6, "G" => 7, "G#" => 8, "A" => 9, "A#" => 10, "B" => 11,
        _ => 0, // Default to C
    };
    let duration_minutes = app_state.length.split_whitespace().next().unwrap_or("5").parse::<f32>().unwrap_or(5.0);
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
    let num_beats_per_chord = rng.gen_range(2..=4); // Corrected based on rand docs
    let chord_duration: f32 = num_beats_per_chord as f32 * sec_per_beat;
    let samples_per_chord = (chord_duration * SAMPLE_RATE_AUDIO_GEN as f32) as usize;

    // Call get_melody and get_bass_line with their original signatures (no &mut rng)
    let melody = melodies::get_melody(style, root_note, duration_seconds as u32, sec_per_beat, actual_generated_seed);
    let (chord_sequence, chord_root_notes) = match style.to_lowercase().as_str() {
        "blues" => play_progression(String::from("blues"), root_note, chord_duration),
        "pop" => play_progression(String::from("pop"), root_note, chord_duration),
        "jazz" => play_progression(String::from("jazz"), root_note, chord_duration),
        _ => play_progression(String::from("default"), root_note, chord_duration),
    };
    let melody_len = melody.len();
    let chord_len = chord_sequence.len(); 
    let target_len = melody_len; 
    let bass_line = bass::get_bass_line(style, &chord_root_notes, samples_per_chord, target_len, bpm, actual_generated_seed);
    
    let mut mixed_audio = Vec::with_capacity(target_len);
    let chord_gain = 0.5; let melody_gain = 0.125; let bass_gain = 0.6;
    for i in 0..target_len {
        let chord_sample_val = if chord_len > 0 { chord_sequence.get(i % chord_len).copied().unwrap_or(0.0) * chord_gain } else { 0.0 };
        let melody_sample_val = melody.get(i).copied().unwrap_or(0.0) * melody_gain;
        let bass_sample_val = bass_line.get(i).copied().unwrap_or(0.0) * bass_gain;
        mixed_audio.push(melody_sample_val + chord_sample_val + bass_sample_val);
    }
    if !mixed_audio.is_empty() {
        let max_abs_val = mixed_audio.iter().fold(0.0f32, |max, &val| max.max(val.abs()));
        if max_abs_val > 1.0 { 
            for sample in &mut mixed_audio { *sample /= max_abs_val; } 
        }
    }
    
    (mixed_audio, SAMPLE_RATE_AUDIO_GEN, actual_generated_seed)
}

// This is the main function that will be called to start music playback in a thread
pub fn run_music_service(initial_app_state: AppState, receiver: CrossbeamReceiver<MusicControl>, progress_sender: CrossbeamSender<MusicProgress>) {
    const SAMPLE_RATE_PROGRESS: f32 = 44100.0; // For progress calculation (as f32)

    thread::spawn(move || {
        let mut player = MusicPlayer::new(receiver);
        let current_app_state_for_generation = initial_app_state;
        let actual_seed_for_current_song: u64;

        // Initial audio generation based on initial_app_state
        {
            let (audio_data, sample_rate, seed) = generate_audio_from_state(&current_app_state_for_generation);
            actual_seed_for_current_song = seed;
            player.play_audio(audio_data, sample_rate); // Prepares sink, remains paused
            let _ = progress_sender.send(MusicProgress { // Send initial state
                current_samples: 0,
                total_samples: player.total_samples,
                actual_seed: actual_seed_for_current_song,
            });

            // Auto-start playback for the initial track
            if player.total_samples > 0 { // Ensure there's something to play
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
                            let elapsed_since_last_play = player.playback_start_time.unwrap().elapsed();
                            player.samples_played_at_pause += (elapsed_since_last_play.as_secs_f32() * SAMPLE_RATE_PROGRESS) as u64;
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
                            (&player.current_audio_data, player.current_sample_rate) {
                            
                            // Clone the audio data to pass to play_audio
                            let audio_data_clone = audio_data_ref.clone();
                            player.play_audio(audio_data_clone, sample_rate_val);
                            
                            // After play_audio, the sink is paused by default, so we need to resume it.
                            // Also, play_audio resets progress, so playback_start_time needs to be set here.
                            player.samples_played_at_pause = 0; // Redundant as play_audio does this
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

            // Progress Reporting (logic remains largely the same)
            if player.total_samples > 0 && !player.should_terminate {
                let mut current_display_samples = player.samples_played_at_pause;
                if !player.sink.is_paused() && player.playback_start_time.is_some() {
                    let elapsed_since_current_play = player.playback_start_time.unwrap().elapsed();
                    current_display_samples += (elapsed_since_current_play.as_secs_f32() * SAMPLE_RATE_PROGRESS) as u64;
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

// Function to parse a song ID string into an AppState
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

    let mut app_state = AppState::default();
    app_state.scale = scale;
    app_state.style = style;
    app_state.bpm = bpm_str; 
    app_state.length = length_in_mins;
    app_state.seed = seed_str; 

    Ok(app_state)
}


