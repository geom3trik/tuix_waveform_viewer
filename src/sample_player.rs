use crate::audio_file::AudioFile;
use crate::audio_stream::PlaybackContext;
use basedrop::{Collector, Handle, Shared};
use ringbuf::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

enum PlayerState {
    Playing,
    Stopped,
}

enum Message {
    Seek(f64),
    Scrub(f64),
    Play,
    Stop,
    SetActive(usize, bool),
    NewFile(Shared<AudioFile>),
    Volume(f32),
}

pub struct SamplePlayer {
    pub file: Option<Shared<AudioFile>>,
    active: [bool; 32],
    playhead: Arc<AtomicUsize>,
    state: PlayerState,
    rx: Consumer<Message>,
    volume: f32,
}

pub struct SamplePlayerController {
    tx: Producer<Message>,
    playhead: Arc<AtomicUsize>,
    collector: Handle,
    sample_rate: Option<f64>,
    num_channels: Option<usize>,
    num_samples: Option<usize>,
    pub file: Option<Shared<AudioFile>>,
}

/// create a new sample player and its controller
pub fn sample_player(c: &Collector) -> (SamplePlayer, SamplePlayerController) {
    let playhead = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = RingBuffer::new(2048).split();
    (
        SamplePlayer {
            file: None,
            active: [true; 32],
            playhead: playhead.clone(),
            state: PlayerState::Stopped,
            rx,
            volume: 1.0,
        },
        SamplePlayerController {
            tx,
            playhead: playhead.clone(),
            collector: c.handle(),
            sample_rate: None,
            num_channels: None,
            num_samples: None,
            file: None,
        },
    )
}

impl SamplePlayer {
    pub fn playhead(&self) -> usize {
        self.playhead.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn advance(&mut self, context: &mut PlaybackContext) {
        
        while let Some(msg) = self.rx.pop() {
            match msg {
                Message::Seek(pos) => {
                    if let Some(f) = &self.file {
                        self.playhead.store(
                            ((f.sample_rate * pos) as usize).min(f.num_samples),
                            Ordering::SeqCst,
                        );
                    }
                }
                Message::NewFile(file) => {
                    self.file = Some(file);
                }
                Message::Scrub(_) => {
                    //todo...
                }
                Message::SetActive(channel, active) => {
                    self.active[channel] = active;
                }
                Message::Play => self.state = PlayerState::Playing,
                Message::Stop => self.state = PlayerState::Stopped,
                Message::Volume(val) => self.volume = val,
            }
        }

        if let PlayerState::Stopped = self.state {
            return;
        }

        if let Some(file) = &self.file {
            if self.playhead() >= file.num_samples {
                self.state = PlayerState::Stopped;
                return;
            }
            for channel in 0..context.num_channels.max(file.num_channels) {
                if !self.active[channel] {
                    continue;
                }
                let start = channel * file.num_samples + self.playhead().min(file.num_samples);
                let end = channel * file.num_samples
                    + (self.playhead() + context.buffer_size).min(file.num_samples);
                context.get_output(channel)[0..(end - start)]
                    .copy_from_slice(&file.data[start..end]);
                context.get_output(channel)[0..(end - start)].iter_mut().for_each(|sample| *sample = *sample * self.volume);
            }
            self.playhead
                .fetch_add(context.buffer_size, Ordering::SeqCst);
        }
    }
}

#[allow(dead_code)]
impl SamplePlayerController {
    pub fn sample_rate(&self) -> Option<f64> {
        self.sample_rate
    }
    pub fn duration_samples(&self) -> Option<usize> {
        self.num_samples
    }
    pub fn num_channels(&self) -> Option<usize> {
        self.num_channels
    }
    fn send_msg(&mut self, message: Message) {
        let mut e = self.tx.push(message);
        while let Err(message) = e {
            e = self.tx.push(message);
        }
    }
    pub fn seek(&mut self, seconds: f64) {
        self.send_msg(Message::Seek(seconds));
    }
    pub fn playhead(&self) -> usize {
        self.playhead.load(Ordering::SeqCst)
    }
    pub fn play(&mut self) {
        self.send_msg(Message::Play);
    }
    pub fn stop(&mut self) {
        self.send_msg(Message::Stop);
    }
    pub fn scrub(&mut self, seconds: f64) {
        self.send_msg(Message::Scrub(seconds));
    }
    pub fn set_active(&mut self, channel_index: usize, active: bool) {
        self.send_msg(Message::SetActive(channel_index, active));
    }
    pub fn volume(&mut self, val: f32) {
        self.send_msg(Message::Volume(val));
    }
    pub fn load_file(&mut self, s: &str) {
        let audio_file = Shared::new(
            &self.collector,
            AudioFile::open(s).expect("file does not exist"),
        );
        self.num_samples = Some(audio_file.num_samples);
        self.num_channels = Some(audio_file.num_channels);
        self.sample_rate = Some(audio_file.sample_rate);
        self.file = Some(Shared::clone(&audio_file));
        self.send_msg(Message::NewFile(audio_file));
    }
    pub fn get_magnitude(&self, sample_idx: usize) -> f32 {
        if let Some(file) = &self.file {
            let ldx = sample_idx;
            let rdx = sample_idx + file.num_samples;
            (file.data[ldx].abs() + file.data[rdx].abs()) / 2.0
        } else {
            0.0
        }
    }
}