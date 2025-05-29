![](https://img.shields.io/badge/Rust-%23000000.svg?style=for-the-badge&logo=rust&color=orange&logoColor=white&labelColor=orange)

# 🎵 8BitBeats - 8-Bit Music Generator

## 📌 Project Overview
8BitBeats is a terminal-based 8-bit music generator that allows you to create, customize, and replay chiptune-style music. Generate unique, reproducible tracks using a deterministic algorithm based on your inputs. Easily manage and replay tracks via track IDs.

## 🎯 Features
- **Random Music Generation**: Generate unique 8-bit tracks with adjustable parameters.
- **TUI Interface**: Intuitive terminal UI for an engaging experience.
- **Customizable Inputs**:
  - Scale (C, C#, D, ... B)
  - Style (Pop, Rock, Jazz, etc.)
  - BPM (Beats Per Minute)
  - Length (duration in minutes)
  - Optional seed for controlled randomness
- **Reproducible Tracks**:
  - Each track is assigned a deterministic track ID
  - Regenerate the same track by providing the same inputs or track ID
- **Playback Controls**:
  - Play, Pause, Rewind, Skip, and Fast Forward
  - Load tracks by ID and resume playback
- **Keyboard Shortcuts**:
  - Navigate UI elements with arrow keys
  - [p] Play/Pause, [r] Rewind, [s] Skip, [g] Generate, [q] Quit, [?] Toggle Help

## 🚀 Installation

1. **Clone the repository:**
   ```sh
   git clone https://github.com/DerekCorniello/8BitBeats.git
   cd 8BitBeats
   ```
2. **Build the project:**
   ```sh
   cargo build --release
   ```
3. **Run the application:**
   ```sh
   cargo run --release
   ```

## 🕹️ Usage
- Use the arrow keys to navigate between UI elements.
- Press [g] to generate a new track, [r] to rewind, [s] to skip, [p] to play/pause.
- Enter a track ID to replay a specific song.
- Press [?] to toggle the help menu.
- All controls are visible in the TUI help panel.

## 💾 Reproducibility
- Every generated track is assigned a unique, deterministic ID based on your inputs (scale, style, bpm, length, seed).
- To replay a song, enter its track ID in the loader field and press Enter.

## 🛠️ Dependencies
See `Cargo.toml` for a full list. Major dependencies include:
- `rodio` (audio playback)
- `rand` (randomness)
- `ratatui`, `crossterm` (terminal UI)
- `rust-music-theory` (music theory)
- `crossbeam-channel` (threading)

## 📄 License
This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## 🤝 Contributing
Pull requests, bug reports, and feature suggestions are welcome! Please open an issue or submit a PR on GitHub.

## Connect with Me!
[![LinkedIn](https://img.shields.io/badge/LinkedIn-%230A66C2.svg?style=for-the-badge&logo=linkedin&logoColor=white)](https://www.linkedin.com/in/derek-corniello)
[![GitHub](https://img.shields.io/badge/GitHub-%23121011.svg?style=for-the-badge&logo=github&logoColor=white)](https://github.com/derekcorniello)
[![X](https://img.shields.io/badge/X-%231DA1F2.svg?style=for-the-badge&logo=x&logoColor=white)](https://x.com/derekcorniello)