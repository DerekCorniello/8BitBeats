use crate::melodies;
use crate::progs;
use crate::tui;

use rand::Rng;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink, Source};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tui::AppState;

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

pub enum MusicControl {
    Pause,
    Resume,
    Terminate,
}

pub struct MusicPlayer {
    receiver: Receiver<MusicControl>,
    paused: bool,
    running: bool,
}

impl MusicPlayer {
    pub fn new(receiver: Receiver<MusicControl>) -> Self {
        MusicPlayer {
            receiver,
            paused: false,
            running: true,
        }
    }

    pub fn check_control(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(MusicControl::Pause) => {
                    println!("Received Pause command");
                    self.paused = true;
                }
                Ok(MusicControl::Resume) => {
                    println!("Received Resume command");
                    self.paused = false;
                }
                Ok(MusicControl::Terminate) => {
                    println!("Received Terminate command");
                    self.running = false;
                    self.paused = false;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    println!("Channel disconnected");
                    self.running = false;
                    break;
                }
            }
        }
    }
    pub fn should_play(&self) -> bool {
        self.running && !self.paused
    }

    pub fn should_continue(&self) -> bool {
        self.running
    }
}

// Set up the global sender
static MUSIC_SENDER: OnceLock<Mutex<Option<mpsc::Sender<MusicControl>>>> = OnceLock::new();

pub fn get_music_sender() -> &'static Mutex<Option<mpsc::Sender<MusicControl>>> {
    MUSIC_SENDER.get_or_init(|| Mutex::new(None))
}

// Control functions that can be called from your frontend
pub fn pause_music() -> Result<(), &'static str> {
    let sender = get_music_sender().lock().unwrap();
    if let Some(tx) = &*sender {
        println!("Sending Pause command");
        tx.send(MusicControl::Pause)
            .map_err(|_| "Failed to send pause command")
    } else {
        Err("No music is currently playing")
    }
}

pub fn resume_music() -> Result<(), &'static str> {
    let sender = get_music_sender().lock().unwrap();

    if let Some(tx) = &*sender {
        println!("Sending Resume command");
        tx.send(MusicControl::Resume)
            .map_err(|_| "Failed to send resume command")
    } else {
        Err("No music is currently playing")
    }
}

pub fn stop_music() -> Result<(), &'static str> {
    let sender = get_music_sender().lock().unwrap();

    if let Some(tx) = &*sender {
        tx.send(MusicControl::Terminate)
            .map_err(|_| "Failed to send terminate command")
    } else {
        Err("No music is currently playing")
    }
}

// Function to start music in a separate thread
pub fn start_music_in_thread(mut state: AppState) -> Result<(), &'static str> {
    let sender = get_music_sender().lock().unwrap();
    if sender.is_some() {
        return Err("Music is already playing");
    }
    drop(sender);
    thread::spawn(move || {
        // types are enforced on tui
        // therefore parsing should be fine
        if state.seed == "" {
            let mut rng = rand::rng();
            state.seed = rng.random::<u64>().to_string();
        }
        play_music(
            match state.scale.to_owned().as_str() {
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
                _ => 0,
            },
            str::parse::<u32>(state.bpm.as_str()).unwrap(),
            state
                .length
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<f32>()
                .unwrap(),
            state.style.as_str(),
            str::parse::<u64>(state.seed.as_str()).unwrap(),
        );
    });

    Ok(())
}

pub fn play_music(root_note: u8, bpm: u32, duration: f32, style: &str, seed: u64) {
    const SAMPLE_RATE: u32 = 44100; // CD-quality audio (44.1 kHz)
    let sec_per_beat: f32 = 60.0 / bpm as f32;
    let chord_duration: f32 = 4.0 * sec_per_beat;

    let (tx, rx) = mpsc::channel();

    // Store the sender globally
    {
        let mut sender = get_music_sender().lock().unwrap();
        *sender = Some(tx);
    }

    // Create the player with the receiver
    let mut player = MusicPlayer::new(rx);
    let melody = melodies::get_melody(style, root_note, duration as u32 * 60, bpm, seed);

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
    let mut last_update = Instant::now();
    while player.should_continue() {
        let now = Instant::now();
        let delta_time = now.duration_since(last_update).as_secs_f32();
        last_update = now;
        player.check_control();

        if player.should_play() {
            audio_sink.play();
        }
        // Sleep a bit to reduce CPU usage
        thread::sleep(Duration::from_millis(1));
    }
}
