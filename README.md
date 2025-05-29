# 🎵 8BitBeats - 8-Bit Music Generator

## 📌 Project Overview
8BitBeats is a terminal-based 8-bit music generator that allows users to create, customize, and replay chiptune-style music. The tool generates random, yet reproducible, tracks using a deterministic algorithm based on user-defined inputs. It also provides a simple way to manage and replay previously generated tracks via track keys.

## 🎯 Project Features
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
- Playback Controls:
  - Play, Pause, Rewind, and Skip functionalities

## 🔧 Implementation Plan

### 1. Track Generation Algorithm
- Combine user inputs (scale, BPM, length, seed) into a structured format.
- Map random values to musical notes, rhythms, and instruments.
- Output as a sequence of tones that form a chiptune-style track.

### 2. Track Playback & Controls
- Provide TUI controls for play, pause, rewind, and skip.

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
