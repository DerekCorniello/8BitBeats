# 🎵 8BitBeats - 8-Bit Music Generator

## 📌 Project Overview
8BitBeats is a terminal-based 8-bit music generator that allows users to create, customize, and replay chiptune-style music. The tool generates random, yet reproducible, tracks using a deterministic algorithm based on user-defined inputs. It also provides a simple way to manage and replay previously generated tracks via track keys.

## 🎯 Project Goals

### Core Features
- Random Music Generation: Users can generate unique 8-bit tracks with adjustable parameters.
- TUI Interface: Provide an intuitive terminal-based UI for an engaging user experience.
- Customizable Inputs:
  - Scale (e.g., Major, Minor, Pentatonic, etc.)
  - BPM (Beats Per Minute)
  - Length (duration)
  - Optional seed for controlled randomness
- Reproducible Tracks:
  - Each track is assigned a deterministic track ID.
  - Users can regenerate the same track by providing the same inputs or the track ID.
- Looping & Playback Controls:
  - Ability to loop a track continuously
  - Play, Pause, Rewind, and Skip functionalities
- Track History & Management:
  - Save and retrieve previously generated tracks
  - Load a track using its track ID

### Extended Features (Possible Enhancements)
- Saved Tracks: Allow the user to save a local file with their saved IDs so we can replay from there.
- Genre Selection: Generate music inspired by different chiptune styles (e.g., upbeat arcade, RPG, spooky, etc.).
- Instrument Selection: Allow users to pick different 8-bit sound presets.
- Export Options: Save generated tracks as `.wav` or `.mp3` files.

## 🎨 Terminal UI (TUI) Mockup
```
 █████╗       ██████╗ ██╗████████╗   ██████╗ ███████╗ █████╗ ████████╗███████╗
██╔══██╗      ██╔══██╗██║╚══██╔══╝   ██╔══██╗██╔════╝██╔══██╗╚══██╔══╝██╔════╝
╚█████╔╝█████╗██████╔╝██║   ██║█████╗██████╔╝█████╗  ███████║   ██║   ███████╗
██╔══██╗╚════╝██╔══██╗██║   ██║╚════╝██╔══██╗██╔══╝  ██╔══██║   ██║   ╚════██║
╚█████╔╝      ██████╔╝██║   ██║      ██████╔╝███████╗██║  ██║   ██║   ███████║
 ╚════╝       ╚═════╝ ╚═╝   ╚═╝      ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚══════╝
                                                                              
                          ♪ ♫ ♪  The 8 Bit Music DJ  ♪ ♫ ♪                          
┌────────────────────────────────────────────────────────────────────────────┐
│          Now Playing: [Generated Track ID] - [01:15 / 02:30]               │
│                                                                            │
│         Progress: ██████████████████░░░░░░░░░░░░░░░░░░░   [50%]            │
│                                                                            │
│         [<< Rewind]  [▶ Play/Pause]  [>> Skip]  [↺ Enable Loop]            │
├────────────────────────────────────────────────────────────────────────────┤
│                           ♬ Create New Track                               │
│                                                                            │
│           Scale: [ Major    ▼] Style: [ 8-bit    ▼]                        │
│                                                                            │
│            BPM: [120   ] Length: [30 sec  ]                                │
│                                                                            │
│                        Seed (optional): [______]                           │
│                                                                            │
│                              [♫ Generate]                                  │
├────────────────────────────────────────────────────────────────────────────┤
│                           ⌾ Load Track by ID                               │
│                                                                            │
│           Track ID: [________________________________] [↓ Load]            │
└────────────────────────────────────────────────────────────────────────────┘
```

## 🔢 How Track IDs Are Generated
To ensure reproducibility, each track's unique ID is derived from user inputs and a hash function. The process follows these steps:

1. **Collect Inputs**:
   - Scale: "Major"
   - BPM: 120
   - Length: 30 seconds
   - (Optional) Seed: 42

   Note: these inputs can also just be random, to generate random songs completely

2. **Create a String Representation**:
   ```
   track_seed = rand() or 42
   track_string = "0-120-30"
   track_id = track_seed + "-" + track_string
   track_id = "42-0-120-30"
   ```

3. **Use the Track ID to Seed Music Generation**:
   - This ensures that the same input parameters always generate the same track, allowing for easy retrieval and playback.

## 🔧 Implementation Plan

### 1. Track Generation Algorithm
- Combine user inputs (scale, BPM, length, seed) into a structured format.
- Map random values to musical notes, rhythms, and instruments.
  - Probably a good idea to have some default patterns (scales, hops, etc)
- Output as a sequence of tones that form a chiptune-style track.

### 2. Track Playback & Controls
- Provide TUI controls for play, pause, rewind, and skip.
- Enable looping functionality.

### 3. Track Saving & Replaying
- Save generated track IDs and their metadata in a JSON file.
- Allow users to input a track ID to regenerate the exact same track.

### 4. TUI Development
- Implement navigation and keyboard shortcuts for ease of use.

## 📦 Dependencies and Their Use Cases

This project uses several Rust crates to handle music generation, playback, terminal UI, and data persistence. Below is a breakdown of the dependencies by their purpose.

### Audio Processing and Playback
- **dasp** – A digital signal processing (DSP) library used for handling and manipulating audio data.  
- **rodio** – A high-level audio playback library for playing generated 8-bit music.  

### Music Theory and Composition
- **rust-music-theory** – Provides tools for handling scales, chords, and other music theory concepts needed for procedural generation.  

### Terminal UI (TUI)
- **ratatui** – A Rust library for building rich terminal user interfaces, used for the interactive experience of *8BitBeats*.  
- **crossterm** – Handles terminal input/output for navigation and interactivity within the TUI.  

### Data Storage and Reproducibility
- **serde** – A framework for serializing and deserializing structured data, used to store and load track configurations.  
- **serde_json** – Enables saving and reading JSON files for user-defined settings and history tracking.  


## Task Division:

- TUI
    - Enable RR/PP/FF/Loop Capabilities
    - Send Generation Data
    - Send Signal to load track and play it

- Music Gen
    - Given a set of numbers, figure out how to turn them into music
    - Will definitely have the numbers of the scale, bpm, and length, even if they were randomly generated
    - Use scales, chords, progressions, etc to create the music

- Data Passing (TUI inputs -> randomize -> Music Gen)
    - Apply randomness to inputs, pass along to music generation
    - Figure out how to bind certain random values to a range of possible values needed in music generation

- Playing the Music
    - Enable a way to pause, play
    - Interrupt the song and FF/RR as needed
    - Interrupt the song to play the next generated song (if generate or load is pressed)

- [Stretch] Create a playlist
    - Save song IDs to a named config file
    - Load and play the saved songs
    - Implement shuffle
