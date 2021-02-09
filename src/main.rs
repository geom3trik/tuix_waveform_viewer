// Unicode for Icons
const ICON_TO_START: &str = "\u{23ee}";
const ICON_PLAY: &str = "\u{25b6}";
const ICON_PAUSE: &str = "\u{2389}";
const ICON_STOP: &str = "\u{25a0}";
const ICON_TO_END: &str = "\u{23ed}";
const ICON_PLUS: &str = "\u{2b}";
const ICON_MINUS: &str = "\u{2d}";
const ICON_SOUND: &str = "\u{1f50a}";
const ICON_MUTE: &str = "\u{1f507}";
const ICON_MAGNET: &str = "\u{e7a1}";
const ICON_LOCK_OPEN: &str = "\u{1f513}";
const ICON_LOCK: &str = "\u{1f512}";



mod audio_file;
mod audio_stream;
mod sample_player;
mod utils;
use audio_stream::audio_stream;
use basedrop::Collector;
use cpal::{PlayStreamError, traits::StreamTrait};
use sample_player::*;

use tuix::*;

use native_dialog::FileDialog;

use std::{cmp::Ordering, println};

use dasp_sample::{Sample, I24};

use femtovg::{
    renderer::OpenGl,
    Canvas,
    Paint,
    Path,
};

pub fn round_up(num: u32, multiple: u32) -> u32 {
    if multiple == 0 {
        return num;
    }

    let remainder = num % multiple;

    if remainder == 0 {
        return num;
    }

    return num + multiple - remainder;
}

// A time value type for displaying the cursor time
pub struct TimeValue(pub f32);

impl std::fmt::Display for TimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.abs() < 1.0 {
            write!(f, "{:.0} ms", self.0 * 1000.0)
        } else if self.0.abs() >= 1.0 && self.0.abs() < 60.0 {
            write!(f, "{:.3} s", self.0)
        } else if self.0.abs() >= 60.0 && self.0.abs() < 3600.0 {
            write!(f, "{:.0} m {:.3} s", (self.0 / 60.0).floor(), self.0 % 60.0)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl From<f32> for TimeValue {
    fn from(src: f32) -> TimeValue {
        TimeValue(src)
    }
}

fn main() -> Result<(), PlayStreamError> {

    // initialize gc
    let gc = Collector::new();

    // Create the sample player and controller
    let (mut player, mut controller) = sample_player(&gc);
    
    // initialize state and begin the stream
    std::thread::spawn(move || {
        
        let stream = audio_stream(move |mut context| {
            player.advance(&mut context);
        });

        // TODO - handle error
        stream.play();

        std::thread::park();
    });

    // Create a tuix application
    let app = Application::new(|win_desc, state, window| {
        
        // Import the stylsheet
        state
            .insert_stylesheet("src/theme.css")
            .expect("Failed to load stylesheet");

        // Set the window background color
        window.set_background_color(state, Color::rgb(40, 40, 40));

        // Create the app widget
        let app_widget = AppWidget::new(gc, controller).build(state, window, |builder| builder.class("app"));

        // Process command line arguments and send a LoadAudioFile event
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            state.insert_event(
                Event::new(AppEvent::LoadAudioFile(args[1].clone())).target(app_widget),
            );
        }

        // Set the window properties
        win_desc
            .with_title("Waveform Viewer")
            .with_inner_size(1000, 600)
    });

    // Start the tuix app event loop
    app.run();

    Ok(())
}

const SAMPLES_PER_PIXEL: [usize; 11] = [
    4410, 1764, 882, 441, 147, 63, 49, 21, 9, 7, 3
];

pub enum AppError {
    FileReadError,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelMode {
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitsMode {
    Linear,
    Decibel,
}

// Waveform viewer events
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    OpenFileDialog,
    LoadAudioFile(String),
    SwicthChannel(ChannelMode),
    SwitchUnits(UnitsMode),
    SetZoomLevel(usize),
    FollowPlayhead(bool),
    Volume(f32),

    Mute(bool),

    Play,
    Pause,
    Stop,
    SeekLeft,
    SeekRight,
}

pub struct AppWidget {
    left_channel: Vec<f32>,
    right_channel: Vec<f32>,
    zoom_level: usize,
    scroll_pos: usize,

    samples_per_pixel: usize,

    zoom_pos_pixel: f32,

    mute: f32,
    volume: f32,

    start: usize,
    end: usize,
    zoom_pos: usize,
    playhead: usize,

    channel_mode: ChannelMode,
    units_mode: UnitsMode,

    time_label: Entity,
    value_label: Entity,
    playhead_label: Entity,

    waveview: Entity,

    // Player data
    is_playing: bool,
    num_of_samples: usize,
    num_of_channels: usize,
    sample_rate: f64,

    play_button: Entity,

    collector: Collector,
    controller: SamplePlayerController,

    random_animation: usize,
    follow_playhead: bool,

}

impl AppWidget {
    pub fn new(collector: Collector, controller: SamplePlayerController) -> Self {
        Self {
            left_channel: Default::default(),
            right_channel: Default::default(),
            zoom_level: 3,
            scroll_pos: 0,

            mute: 1.0,
            volume: 1.0,

            samples_per_pixel: 441,

            zoom_pos_pixel: 0.0,

            num_of_samples: 0,
            num_of_channels: 0,
            sample_rate: 0.0,

            start: 0,
            end: 0,
            zoom_pos: 0,
            playhead: 0,

            channel_mode: ChannelMode::Left,
            units_mode: UnitsMode::Linear,

            time_label: Entity::null(),
            value_label: Entity::null(),
            playhead_label: Entity::null(),

            waveview: Entity::null(),

            is_playing: false,

            play_button: Entity::null(),

            collector,
            controller,

            random_animation: std::usize::MAX,
            follow_playhead: true,
        }
    }
}

impl AppWidget {
    // Opens the wav file, reads the audio samples into left and right audio buffers
    
    fn read_audio(&mut self, filename: &str) -> Result<(), AppError> {
        let reader = hound::WavReader::open(filename).expect("Failed to open wav file");

        let spec = reader.spec();
        let audio: Vec<f32> = match (spec.bits_per_sample, spec.sample_format) {
            (24, hound::SampleFormat::Int) => reader
                .into_samples::<i32>()
                .filter_map(Result::ok)
                .map(|x| I24::new(x).unwrap().to_sample::<f32>())
                .collect(),
            (16, hound::SampleFormat::Int) => reader
                .into_samples::<i16>()
                .filter_map(Result::ok)
                .map(|x| x.to_sample::<f32>())
                .collect(),
            _ => {
                return Err(AppError::FileReadError);
            }
        };

        let buffer_size = audio.len();

        self.left_channel = vec![0.0; buffer_size / 2];
        self.right_channel = vec![0.0; buffer_size / 2];

        for (interleaved_samples, (left, right)) in audio.chunks(2).zip(
            self.left_channel
                .iter_mut()
                .zip(self.right_channel.iter_mut()),
        ) {
            *left = interleaved_samples[0];
            *right = interleaved_samples[1];
        }

        Ok(())
    }

    // Draw the audio waveforms
    fn draw_channel(
        &self,
        state: &mut State,
        entity: Entity,
        data: &[f32],
        posy: f32,
        height: f32,
        canvas: &mut Canvas<OpenGl>,
    ) {
        let x = state.data.get_posx(self.waveview);
        let y = posy;
        let w = state.data.get_width(self.waveview);
        let h = height;

        if data.len() > 0 {
            let audio = &data[self.start as usize..self.end as usize];

            // Fill the background
            let mut path = Path::new();
            path.rect(x, y, w, h);
            canvas.fill_path(
                &mut path,
                Paint::color(femtovg::Color::rgba(40, 40, 40, 255)),
            );

            // Minimum time
            let start_time = self.start as f32 / self.sample_rate as f32;
            let end_time = self.end as f32 / self.sample_rate as f32;

            let total_time = end_time - start_time;

            if total_time > 0.0 && total_time < 60.0 {
                let pixels_per_time_step = w / total_time;
            }

            // Draw grid
            let pixels_per_second = w / total_time;

        
            let first = round_up(start_time as u32, 1);
            let last = round_up(end_time as u32, 1);

            for n in (first..last+1) {
                let sample = self.sample_rate as f32 * n as f32 - self.start as f32;
                let pixel = ((1.0 / self.samples_per_pixel as f32) * sample).round();
                let mut path = Path::new();
                path.move_to(x + pixel, y + 20.0);
                path.line_to(x + pixel, y + 30.0);
                let mut paint = Paint::color(femtovg::Color::rgba(90, 90, 90, 255));
                paint.set_line_width(1.0);
                paint.set_anti_alias(false);
                canvas.stroke_path(&mut path, paint);
            }

            // Create two paths for min/max and rms
            let mut path1 = Path::new();
            let mut path2 = Path::new();

            // Move to the center of the drawing region
            path1.move_to(x, y + h / 2.0);
            path2.move_to(x, y + h / 2.0);

            // TODO
            // Sample-level drawing
            // if self.samples_per_pixel < 1.0 {
            //     for pixel in 0..w as u32 {
            //         let pixels_per_sample = (1.0 / self.samples_per_pixel) as u32;

            //         if pixel % pixels_per_sample == 0 {
            //             let sample =
            //                 self.start + (self.samples_per_pixel * pixel as f32).floor() as usize;
            //             path1.move_to(x + (pixel as f32), y + h / 2.0);
            //             path1.line_to(
            //                 x + (pixel as f32),
            //                 y + h / 2.0 - data[sample as usize] * h / 2.0,
            //             );

            //             path2.move_to(x + (pixel as f32), y + h / 2.0);
            //             path2.line_to(
            //                 x + (pixel as f32),
            //                 y + h / 2.0 - data[sample as usize] * h / 2.0,
            //             );
            //         }
            //     }
            // } else {

                let mut chunks = audio.chunks(self.samples_per_pixel);

                for chunk in 0..w as u32 {
                    if let Some(c) = chunks.next() {
                        let v_min = *c
                            .iter()
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                            .unwrap();
                        let v_max = *c
                            .iter()
                            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                            .unwrap();
                        let v_mean: f32 = (c.iter().map(|s| s*s).sum::<f32>() / c.len() as f32).sqrt();

                        match self.units_mode {
                            UnitsMode::Decibel => {
                                let v_min_db =
                                    1.0 + (20.0 * v_min.abs().log10()).max(-60.0) / 60.0;
                                let v_max_db =
                                    1.0 + (20.0 * v_max.abs().log10()).max(-60.0) / 60.0;

                                let v_mean_db =
                                    1.0 + (20.0 * v_mean.abs().log10()).max(-60.0) / 60.0;

                                let v_min_db = if v_min < 0.0 { -v_min_db } else { v_min_db };

                                let v_max_db = if v_max < 0.0 { -v_max_db } else { v_max_db };

                                let v_mean_db = if v_mean < 0.0 { -v_mean_db } else { v_mean_db };



                                path1.line_to(x + (chunk as f32), y + h / 2.0 - v_min_db * h / 2.0);
                                path1.line_to(x + (chunk as f32), y + h / 2.0 - v_max_db * h / 2.0);

                                path2.move_to(x + (chunk as f32), y + h / 2.0 + v_mean_db * h / 2.0);
                                path2.line_to(x + (chunk as f32), y + h / 2.0 - v_mean_db * h / 2.0);
                            }

                            UnitsMode::Linear => {
                                path1.line_to(x + (chunk as f32), y + h / 2.0 - v_min * h / 2.0);
                                path1.line_to(x + (chunk as f32), y + h / 2.0 - v_max * h / 2.0);

                                path2.move_to(x + (chunk as f32), y + h / 2.0 + v_mean * h / 2.0);
                                path2.line_to(x + (chunk as f32), y + h / 2.0 - v_mean * h / 2.0);
                            }
                        }
                    }
                }
            //}

            // Draw min/max paths
            let mut paint = Paint::color(femtovg::Color::rgba(50, 50, 255, 255));
            paint.set_line_width(1.0);
            paint.set_anti_alias(false);
            canvas.stroke_path(&mut path1, paint);

            // Draw rms paths
            if self.zoom_level < 5 {
                let mut paint = Paint::color(femtovg::Color::rgba(80, 80, 255, 255));
                paint.set_line_width(1.0);
                paint.set_anti_alias(false);
                canvas.stroke_path(&mut path2, paint);                
            }


            // TODO
            // Draw cursor
            // let mut path = Path::new();
            // path.move_to(x + self.zoom_pos_pixel, y);
            // path.line_to(x + self.zoom_pos_pixel, y + h);
            // paint.set_line_width(1.0);
            // paint.set_anti_alias(false);
            // canvas.fill_path(
            //     &mut path,
            //     Paint::color(femtovg::Color::rgba(255, 50, 50, 255)),
            // );


          

            // Draw selection
            // TODO



            // Draw playhead
            let playhead = self.controller.playhead() as f64;
           
            let pixels_per_sample = 1.0 / self.samples_per_pixel as f32;
            let playheadx = x + pixels_per_sample * (playhead as f32 - self.start as f32);

            let mut path = Path::new();
            if self.follow_playhead && self.start > 0 {
                path.move_to(x + w/2.0, y);
                path.line_to(x + w/2.0, y + h);
            } else {
                path.move_to(playheadx.floor(), y);
                path.line_to(playheadx.floor(), y + h);                
            }

            paint.set_line_width(1.0);
            paint.set_anti_alias(false);
            canvas.fill_path(
                &mut path,
                Paint::color(femtovg::Color::rgba(50, 200, 50, 255)),
            );
        }
    }
}

impl BuildHandler for AppWidget {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        entity.set_flex_grow(state, 1.0);

        state.focused = entity;
        // TEMP - Animations cause the event loop to run continuously so that the timeline cursor moves smoothly
        let animation_state = AnimationState::new()
            .with_duration(std::time::Duration::from_secs(200))
            .with_keyframe((0.0, Color::rgb(0,0,0)))
            .with_keyframe((1.0, Color::rgb(255,255,255)))
            .set_persistent(true);

        self.random_animation = state
            .style
            .border_color
            .insert_animation(animation_state.clone());

        // Header
        let header = Element::new().build(state, entity, |builder| builder.class("header"));
        
        // Used to add space between header and footer but isn't actually visible
        // TODO - create widget that actually displays a waveform
        self.waveview = Element::new().build(state, entity, |builder| {
            builder
                .class("waveview")
                .set_visibility(Visibility::Invisible)
        });

        // Footer
        let footer = Element::new().build(state, entity, |builder| builder.class("footer"));

        // Open file button
        Button::new()
            .on_release(Event::new(AppEvent::OpenFileDialog))
            .build(state, header, |builder| {
                builder
                    .set_text("Open")
                    .set_margin(Length::Pixels(10.0))
                    .class("open")
            });

        // Transpoort controls
        let transport = Element::new().build(state, header, |builder| builder.class("transport"));

            // To start button
            Button::new()
            .on_press(Event::new(AppEvent::SeekLeft).target(entity))
            .build(state, transport, |builder| {
                builder
                    .set_text(ICON_TO_START)
                    .set_font("Icons")
                    .class("first")
            });

            // Play button
            self.play_button = Checkbox::new(true)
                .on_unchecked(Event::new(AppEvent::Play).target(entity))        
                .on_checked(Event::new(AppEvent::Pause).target(entity))
                .with_icon_checked(ICON_PLAY)
                .with_icon_unchecked(ICON_PAUSE)        
                .build(state, transport, |builder| {
                    builder
                        .set_text(ICON_PLAY)
                        .set_font("Icons")
                        .class("play")
            });

            // Stop button
            Button::new()
                .on_press(Event::new(AppEvent::Stop).target(entity)) 
                .build(state, transport, |builder| {
                builder.set_text(ICON_STOP).set_font("Icons")
            });

            // To end button
            Button::new().build(state, transport, |builder| {
                builder
                    .set_text(ICON_TO_END)
                    .set_font("Icons")
                    .class("last")
            });

        self.playhead_label = Label::new("Time: -").build(state, header, |builder| {
            builder.class("info").set_margin(Length::Pixels(10.0))
        });

        //Button::new().build(state, header, |builder| builder.set_text(ICON_SOUND).set_font("Icons").class("volume"));


        Checkbox::new(true)
            .on_unchecked(Event::new(AppEvent::Mute(false)).target(entity))        
            .on_checked(Event::new(AppEvent::Mute(true)).target(entity))
            .with_icon_checked(ICON_SOUND)
            .with_icon_unchecked(ICON_MUTE)        
            .build(state, header, |builder| {
                builder
                    .set_font("Icons")
                    .class("snap")
        });

        let volume_slider = Slider::new()
            .on_change(move |value| Event::new(AppEvent::Volume(value)).target(entity))
            .build(state, header, |builder| builder.class("volume"));


        // Channels selector
        let channels = RadioList::new().build(state, header, |builder| builder.class("checklist"));

            // Left
            RadioButton::new()
                .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Left)).target(entity))
                .build(state, channels, |builder| {
                    builder.set_text("L").class("first")
                    }).set_checked(state, true);

            // Right
            RadioButton::new()
                .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Right)).target(entity))
                .build(state, channels, |builder| builder.set_text("R"));
            
            // Both
            RadioButton::new()
                .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Both)).target(entity))
                .build(state, channels, |builder| {
                    builder
                        .set_text("L + R")
                        .class("last")
                        .set_width(Length::Pixels(60.0))
                });
        
        // // Cursor time
        // self.time_label = Label::new("Time: -").build(state, header, |builder| {
        //     builder.class("info").set_margin(Length::Pixels(10.0))
        // });

        // // Cursor value
        // self.value_label = Label::new("Value: -").build(state, header, |builder| {
        //     builder.class("info").set_margin(Length::Pixels(10.0))
        // });

        // Units selector
        let units = RadioList::new().build(state, header, |builder| builder.class("checklist"));

            // Linear
            RadioButton::new()
                .on_checked(Event::new(AppEvent::SwitchUnits(UnitsMode::Linear)).target(entity))
                .build(state, units, |builder| {
                    builder.set_text("Mag").class("first")
                }).set_checked(state, true);

            // Decibels
            RadioButton::new()
                .on_checked(Event::new(AppEvent::SwitchUnits(UnitsMode::Decibel)).target(entity))
                .build(state, units, |builder| builder.set_text("dB").class("last"));
        


        // FOOTER
        Checkbox::new(false)
            .on_unchecked(Event::new(AppEvent::FollowPlayhead(false)).target(entity))        
            .on_checked(Event::new(AppEvent::FollowPlayhead(true)).target(entity))
            .with_icon_checked(ICON_LOCK)
            .with_icon_unchecked(ICON_LOCK_OPEN)        
            .build(state, footer, |builder| {
                builder
                    .set_font("Icons")
                    .class("snap")
        });

        // Zoom Controls
        let zoom_controls =
            Element::new().build(state, footer, |builder| builder.class("zoom_controls"));

        Button::with_label(ICON_MINUS).build(state, zoom_controls, |builder| {
            builder
                .set_font("Icons")
                .class("zoom")
                .class("first")
        });

        let zoom_levels_dropdown = ZoomDropdown::new()
            .build(state, zoom_controls, |builder| builder.class("zoom"));

        let zoom_levels_list = RadioList::new().build(state, zoom_levels_dropdown, |builder| {
            builder
                .class("checklist")
                .set_flex_direction(FlexDirection::Column)
        });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(10)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("147X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(9)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("63X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(8)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("49X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(7)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("21X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(6)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("9X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(5)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("7X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(4)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("3X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(3)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("1X").class("zoom")
            }).set_checked(state, true);

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(2)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.5X").class("zoom")
            });

        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(1)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.25X").class("zoom")
            });
        
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(0)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.1X").class("zoom")
            });

        // TODO
        // RadioButton::new()
        //     .on_checked(Event::new(AppEvent::SetZoomLevel(0)))
        //     .build(state, zoom_levels_list, |builder| {
        //         builder.set_text("FIT").class("zoom")
        //     });

        Button::with_label(ICON_PLUS).build(state, zoom_controls, |builder| {
            builder
                .set_font("Icons")
                .class("zoom")
                .class("last")
        });

        entity
    }
}

impl EventHandler for AppWidget {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        
        // Handle window events
        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {
                
                // When the window width changes, display more or less of the waveform
                WindowEvent::GeometryChanged(_) => {
                    if event.target == entity {
                        let total_samples =
                        (state.data.get_width(entity) * self.samples_per_pixel as f32) as i32;
                        self.end = self.start + (total_samples as usize).min(self.num_of_samples);
                    }
                }

                // Clicking on the waveform moves the playhead to that position
                WindowEvent::MouseDown(button) => {
                    if event.target == entity {
                        if *button == MouseButton::Left {
                            self.controller.seek(self.zoom_pos as f64 / self.sample_rate);
                        }
                    }
                }

                // Moving the mouse moves the cursor position
                WindowEvent::MouseMove(x, _) => {
                    if event.target == entity {
                        if self.num_of_samples > 0 {
                            self.zoom_pos_pixel = *x - state.data.get_posx(entity);

                            self.zoom_pos = self.start
                                + (self.samples_per_pixel as f32 * self.zoom_pos_pixel) as usize;

                            if self.zoom_pos >= self.num_of_samples / self.num_of_channels {
                                self.zoom_pos = self.num_of_samples / self.num_of_channels - 1;
                            }

                            state.insert_event(Event::new(WindowEvent::Redraw));

                            // Update the time and value display
                            // TODO
                            // if self.left_channel.len() > 0 {
                            //     let time = self.zoom_pos as f32 / 44100.0;
                            //     let time_value: TimeValue = time.into();
                            //     let time_string = format!("Time: {}", time_value);
                            //     self.time_label.set_text(state, &time_string);

                            //     match self.units_mode {
                            //         UnitsMode::Linear => {
                            //             let value = self.left_channel[self.zoom_pos as usize];
                            //             let value_string = format!("Value: {:+.2e}", value);
                            //             self.value_label.set_text(state, &value_string);
                            //         }

                            //         UnitsMode::Decibel => {
                            //             let value = 10.0
                            //                 * self.left_channel[self.zoom_pos as usize].abs().log10();
                            //             let value_string = format!("Value: {:.2} dB", value);
                            //             self.value_label.set_text(state, &value_string);
                            //         }
                            //     }
                            // }   
                        }
                        
                    }
                }

                // Scrolling the mouse will pan the waveform, scrolling with ctrl will zoom the waveform at the cursor
                WindowEvent::MouseScroll(_, y) => {
                    if *y > 0.0 {
                        if state.modifiers.ctrl {
                            if self.zoom_level != SAMPLES_PER_PIXEL.len() - 1 {
                                self.zoom_level += 1;
                            }

                            self.samples_per_pixel = SAMPLES_PER_PIXEL[self.zoom_level];

                            let width = state.data.get_width(entity);

                            // Total number of samples for the whole view width
                            let total_samples = (state.data.get_width(entity)
                                * self.samples_per_pixel as f32) as usize;

                            let mut new_start = 0;
                            let mut new_end = total_samples;

                            let offset = self.zoom_pos as i32
                                - (self.zoom_pos_pixel * self.samples_per_pixel as f32) as i32;
                            
                            if offset > 0 {
                                new_start = offset as usize;
                                new_end = total_samples as usize + offset as usize;
                            }

                            self.end = new_end.min(self.num_of_samples - 1);
                            self.start = new_start.max(0).min(self.end);
                        } else {
                            
                            let samples_to_start = self.start;

                            let new_start;
                            let new_end ;

                            if samples_to_start < (self.samples_per_pixel as f32 * 30.0) as usize {
                                new_start = self.start - samples_to_start;
                                new_end = self.end - samples_to_start;
                            } else {
                                new_start = self.start - (self.samples_per_pixel as f32 * 30.0) as usize;
                                new_end =  self.end - (self.samples_per_pixel as f32 * 30.0) as usize;
                            }

                            self.end = new_end.min(self.num_of_samples - 1);
                            self.start = new_start.max(0).min(self.end);

                        }

                        state.insert_event(Event::new(WindowEvent::Redraw));
                        event.consume();
                    } else if *y < 0.0 {
                        if state.modifiers.ctrl {
                            if self.zoom_level != 0 {
                                self.zoom_level -= 1;
                            }

                            self.samples_per_pixel = SAMPLES_PER_PIXEL[self.zoom_level];

                            let total_samples = (state.data.get_width(entity)
                                * self.samples_per_pixel as f32)
                                as usize;

                            let width = state.data.get_width(entity);

                            let mut new_start = 0;
                            let mut new_end = total_samples;


                            let offset = self.zoom_pos as i32
                                - (self.zoom_pos_pixel * self.samples_per_pixel as f32) as i32;
                            
                            if offset > 0 {
                                new_start = offset as usize;
                                new_end = total_samples as usize + offset as usize;
                            }

                            self.end = new_end.min(self.num_of_samples - 1);
                            self.start = new_start.max(0).min(self.end);

                        } else {

                            let samples_to_end = self.num_of_samples - 1 - self.end;

                            let new_start;
                            let new_end;

                            if samples_to_end < (self.samples_per_pixel as f32 * 30.0) as usize {
                                new_start = self.start + samples_to_end;
                                new_end = self.end + samples_to_end;
                            } else {
                                new_start = self.start + (self.samples_per_pixel as f32 * 30.0) as usize;
                                //let sample_end = self.start + state.data.get_width(entity) * self.samples_per_pixel;
                                
                                new_end =  self.end + (self.samples_per_pixel as f32 * 30.0) as usize;
                                // if samle_end < new_end {
                                //     new_end 
                                // }
                            }

                            self.start = new_start.max(0).min(self.end);
                            self.end = new_end.min(self.num_of_samples - 1);
                        }

                        state.insert_event(Event::new(WindowEvent::Redraw));
                        event.consume();
                    }
                }


                WindowEvent::KeyDown(code, key) => {
                    println!("Key: {:?} {:?}", code, key);
                    match code {
                        Code::Space => {
                            if self.is_playing {
                                state.insert_event(Event::new(AppEvent::Pause).target(entity));
                            } else {
                                state.insert_event(Event::new(AppEvent::Play).target(entity));
                            }
                            
                        }

                        Code::KeyS => {
                            state.insert_event(Event::new(AppEvent::Stop).target(entity));
                        }

                        _=> {}
                    }

                    match key {
                        Some(Key::ArrowLeft) => {
                            //println!("Do This");
                            if self.playhead > 0 {
                                let current_time = self.playhead as f64 / self.sample_rate;
                                let new_time = (current_time - 1.0).max(0.0);
                                if self.is_playing {
                                    
                                    self.controller.seek(new_time);
                                } else {
                                    self.playhead -= self.samples_per_pixel;
                                    //println!("playhead: {}", self.playhead);
                                    let current_time = (self.playhead as f64 / self.sample_rate).max(0.0);
                                    self.controller.seek(current_time);
                                }

                                state.insert_event(Event::new(WindowEvent::Redraw));
                            }
                        }

                        Some(Key::ArrowRight) => {

                                let current_time = self.playhead as f64 / self.sample_rate;
                                let new_time = (current_time + 1.0).max(0.0);
                                if self.is_playing {
                                    
                                    self.controller.seek(new_time);
                                } else {
                                    self.playhead += self.samples_per_pixel;
                                    //println!("playhead: {}", self.playhead);
                                    let current_time = (self.playhead as f64 / self.sample_rate).max(0.0);
                                    self.controller.seek(current_time);
                                }

                                state.insert_event(Event::new(WindowEvent::Redraw));
                            
                        }

                        Some(Key::Home) => {
                            state.insert_event(Event::new(AppEvent::SeekLeft).target(entity));
                        }

                        Some(Key::End) => {
                            state.insert_event(Event::new(AppEvent::SeekRight).target(entity));
                        }

                        _=> {}
                    }
                }

                _ => {}
            }
        }

        // Handle application events
        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {

                // Load an audio file specified on the command line
                AppEvent::LoadAudioFile(file_path) => {
                    //self.read_audio(file_path);
                    self.controller.load_file(file_path);


                    if let Some(file) = self.controller.file.as_ref() {
                        self.num_of_channels = file.num_channels;
                        self.sample_rate = file.sample_rate;
                        self.num_of_samples = file.num_samples;
                        println!("Length: {} ", self.num_of_samples);
                    }


                    state.insert_event(Event::new(AppEvent::SetZoomLevel(3)).target(entity));
                }

                // Load an audio file using a file dialog
                AppEvent::OpenFileDialog => {

                    let result = FileDialog::new()
                        .show_open_single_file()
                        .expect("Failed to open file dialog");

                    match result {
                        Some(file_path) => {
                            println!("File path = {:?}", file_path);

                            //self.read_audio(file_path.as_os_str().to_str().unwrap());

                            self.controller.load_file(file_path.as_os_str().to_str().unwrap());
                            self.controller.seek(0.0);
                            self.is_playing = false;

                            if let Some(file) = self.controller.file.as_ref() {
                                self.num_of_channels = file.num_channels;
                                self.sample_rate = file.sample_rate;
                                self.num_of_samples = file.num_samples;
                                println!("Length: {} ", file.num_samples);
                            }


                            state.insert_event(Event::new(AppEvent::SetZoomLevel(3)).target(entity));

                            //let samples_per_pixel = 441.0;

                            // self.zoom_level = 2;
                            // self.start = 0;

                            // self.end = (state.data.get_width(entity) * samples_per_pixel)
                            //     .ceil() as usize;
                            // if self.end > self.num_of_samples - 1 {
                            //     self.end = self.num_of_samples - 1 ;
                            // }

                            // self.start = self.start.min(self.num_of_samples - 1).max(0);
                            // self.end = self.end.min(self.num_of_samples - 1).max(0);
                        }

                        None => {}
                    }

                    event.consume();
                }
                
                // Change the currently visible channel
                AppEvent::SwicthChannel(channel_mode) => {
                    self.channel_mode = channel_mode.clone();
                    state.insert_event(Event::new(WindowEvent::Redraw));
                }

                // Change the display units 
                AppEvent::SwitchUnits(units_mode) => {
                    self.units_mode = units_mode.clone();
                    state.insert_event(Event::new(WindowEvent::Redraw));
                }

                // Change the current zoom level
                // TODO - zoom at cursor/playhead position
                AppEvent::SetZoomLevel(val) => {
                    self.zoom_level = *val;

                    self.samples_per_pixel = SAMPLES_PER_PIXEL[self.zoom_level];

                    let total_samples =
                        (state.data.get_width(entity) * self.samples_per_pixel as f32) as i32;

                    let new_start = 0;
                    let new_end = total_samples as usize;

                    self.start = new_start.max(0);
                    self.end = new_end.min(self.num_of_samples - 1);

                    state.insert_event(Event::new(WindowEvent::Redraw));
                }
                
                // Initiate playback
                AppEvent::Play => {
                    state.insert_event(Event::new(CheckboxEvent::Uncheck).target(self.play_button));
                    self.controller.play();
                    state.style.border_color.play_animation(entity, self.random_animation);
                    self.is_playing = true;
                }

                // Pause playback
                AppEvent::Pause => {
                    state.insert_event(Event::new(CheckboxEvent::Check).target(self.play_button));
                    self.controller.stop();
                    self.is_playing = false;
                }

                // Stop playback
                AppEvent::Stop => {
                    self.controller.stop();
                    self.controller.seek(0.0);
                    state.insert_event(Event::new(CheckboxEvent::Check).target(self.play_button));
                    self.is_playing = false;
                }

                // Move playhead to start
                AppEvent::SeekLeft => {
                    self.controller.seek(0.0);
                    let total_samples =
                        (state.data.get_width(entity) * self.samples_per_pixel as f32) as i32;
                    self.start = 0;
                    self.end = (total_samples as usize).min(self.num_of_samples);
                    self.start = self.start.max(0);
                    self.end = self.end.min(self.num_of_samples - 1);
                }

                // Move playhead to end
                // TODO
                AppEvent::SeekRight => {
                    let end_time = self.num_of_samples as f64 / self.sample_rate;
                    self.controller.seek(end_time);
                }

                AppEvent::FollowPlayhead(val) => {
                    self.follow_playhead = *val;
                }

                AppEvent::Volume(val) => {
                    self.volume = *val;
                    self.controller.volume(*val * self.mute);
                }

                AppEvent::Mute(val) => {
                    if *val {
                        self.mute = 1.0;
                    } else {
                        self.mute = 0.0;
                    }

                    state.insert_event(Event::new(AppEvent::Volume(self.volume)).target(entity));
                }
            }
        }
    }

    // Draw the waveform
    fn on_draw(&mut self, state: &mut State, entity: Entity, canvas: &mut Canvas<OpenGl>) {
        let y = state.data.get_posy(self.waveview);
        let h = state.data.get_height(self.waveview);
        let w = state.data.get_width(self.waveview);

        if let Some(file) = self.controller.file.as_ref() {

            match self.channel_mode {
                ChannelMode::Left => {
                    self.draw_channel(state, entity, &file.data[0..self.num_of_samples], y, h, canvas);
                }

                ChannelMode::Right => {
                    self.draw_channel(state, entity, &file.data[self.num_of_samples..self.num_of_samples*2], y, h, canvas);
                }

                ChannelMode::Both => {
                    self.draw_channel(state, entity, &file.data[0..file.num_samples / 2], y, h / 2.0, canvas);
                    self.draw_channel(
                        state,
                        entity,
                        &file.data[(file.num_samples / 2)+1..file.num_samples],
                        y + h / 2.0,
                        h / 2.0,
                        canvas,
                    );
                }
            }

        }



        // Reset the playhead when it reaches the end of the file
        let playhead = self.controller.playhead() as f64;
        self.playhead = playhead as usize;
        if let Some(file) = self.controller.file.as_ref() {
            if playhead as usize > file.num_samples {
                state.insert_event(Event::new(AppEvent::Stop).target(entity));
            }
        }

        // Update the playhead time display
        if self.is_playing {
            let time = self.playhead as f32 / self.sample_rate as f32;
            let time_value: TimeValue = time.into();
            let time_string = format!("Playhead: {}", time_value);
            self.playhead_label.set_text(state, &time_string);
        }

        // Move the waveform if following the playhead
        if self.is_playing && self.follow_playhead {
            //println!("playhead: {} {}", self.playhead, self.samples_per_pixel as usize);
            if self.playhead % self.samples_per_pixel == 0 {
                //println!("playhead: {}", playhead);
                let playhead_pos_pixel = w / 2.0;
                let offset = self.playhead as i32 - (playhead_pos_pixel * self.samples_per_pixel as f32) as i32;
                let total_samples = (state.data.get_width(entity)
                                    * self.samples_per_pixel as f32) as usize;

                let mut new_start = 0;
                let mut new_end = total_samples;

                if offset > 0 {
                    new_start = offset as usize;
                    new_end = total_samples as usize + offset as usize;
                }

                self.end = new_end.min(self.num_of_samples - 1);
                self.start = new_start.max(0).min(self.end);                
            }

        }
    }
}

// A dropdown container for zoom controls (inherited from a dropdown container)
pub struct ZoomDropdown {
    dropdown: Dropdown,
}

impl ZoomDropdown {
    pub fn new() -> Self {
        Self {
            dropdown: Dropdown::new("1X"),
        }
    }
}

impl BuildHandler for ZoomDropdown {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        self.dropdown.on_build(state, entity).2
    }
}

impl EventHandler for ZoomDropdown {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        self.dropdown.on_event(state, entity, event);

        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {
                AppEvent::SetZoomLevel(val) => {
                    self.dropdown.label.set_text(state, &(((441.0 / SAMPLES_PER_PIXEL[*val] as f32)).to_string() + "X"));
                }

                _=> {}
            }
        }
    }
}
