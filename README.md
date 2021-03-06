# tuix_waveform_viewer
An audio player and waveform viewer for .wav files. Written in Rust.


![screenshot](https://github.com/geom3trik/tuix_waveform_viewer/blob/main/docs/screenshot3.png?raw=true)


## Usage:
For best performance run with release mode:
```Bash
cargo run --release path_to_file.wav
```

## Features:
- [x] Open and load wav file
- [x] View left, right, left + right channels for stereo audio
- [x] View waveform in linear and decibel
- [x] Cursor with time and value display
- [x] Zoom and pan waveform
- [x] Playback controls
- [x] Select a time region for looping
- [x] Navigation pane for easy scrolling
- [ ] Display wav file info
- [ ] Change waveform and backgound colors
- [ ] Menu for changing properties
- [ ] Support for more than 2 channels

## Known Issues:
- Open file dialog blocks on MAC OS causing freeze
- Sample-level display is missing
- Opening a mono file instead of a stereo file crashes
