mod melodies;
mod test;
mod tui;
use crate::tui::Tui;

use ratatui::prelude::CrosstermBackend;
use rodio::buffer::SamplesBuffer;
use rodio::OutputStream;
use rodio::{Sink, Source};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut tui = Tui::new(backend)?;

    tui.setup()?;

    loop {
        tui.draw()?;
        let all_samples =
            melodies::create_custom_melody(0, "diatonic", "major", 4, "simple", 10.0, 120);

        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get output stream");

        let audio_sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        // Prepare audio data and set it to repeat
        let audio_source =
            SamplesBuffer::new(1, 44100, all_samples.clone()).repeat_infinite();

        // Add the audio to the sink
        audio_sink.append(audio_source);
        if !tui.handle_input()? {
            break;
        }
    }

    tui.teardown()?;

    Ok(())
}
