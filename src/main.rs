

mod audio_file;
mod audio_stream;
mod sample_player;
mod utils;

use audio_stream::audio_stream;
use basedrop::Collector;
use cpal::{traits::StreamTrait, PlayStreamError};
use sample_player::*;
mod waveform;
use waveform::*;

mod widgets;
use widgets::*;

use tuix::*;

use image::GenericImageView;

use native_dialog::FileDialog;

use femtovg::{renderer::OpenGl, Canvas, Paint, Path};

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
        if self.0.abs() >= 0.0 && self.0.abs() < 60.0 {
            write!(f, "00''00'{:04.1}", self.0)
        } else if self.0.abs() >= 60.0 && self.0.abs() < 3600.0 {
            write!(
                f,
                "00''{:02}'{:04.1}",
                (self.0 / 60.0).floor(),
                self.0 % 60.0
            )
        } else if self.0.abs() >= 3600.0 && self.0.abs() < 3600.0 {
            write!(
                f,
                "{:02}''{:02}'{:04.1}",
                (self.0 / 3600.0).floor(),
                ((self.0 % 3600.0) / 60.0).floor(),
                self.0 % 60.0
            )
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
    let icon = image::open("icon.png").expect("Failed to find icon");

    // initialize gc
    let gc = Collector::new();

    // Create the sample player and controller
    let (mut player, controller) = sample_player(&gc);

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
            .add_stylesheet("src/theme.css")
            .expect("Failed to load stylesheet");

        // Set the window background color
        window.set_background_color(state, Color::rgb(40, 40, 40));

        // Create the app widget
        let app_widget =
            AppWidget::new(gc, controller).build(state, window, |builder| builder.class("app"));

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
            .with_icon(icon.to_bytes(), icon.width(), icon.height())
    });

    // Start the tuix app event loop
    app.run();

    Ok(())
}
pub enum AppError {
    FileReadError,
}

pub struct AppWidget {
    perf: PerfGraph,
    prevt: std::time::Instant,

    zoom_level: usize,
    samples_per_pixel: usize,

    zoom_pos_pixel: f32,

    mute: f32,
    volume: f32,

    start: usize,
    end: usize,
    zoom_pos: usize,
    playhead: usize,
    cursor: usize,
    select: usize,

    channel_mode: ChannelMode,
    units_mode: UnitsMode,
    play_state: PlayState,

    playhead_label: Entity,

    waveview: Entity,

    // Player data
    is_playing: bool,
    should_loop: bool,

    num_of_samples: usize,
    num_of_channels: usize,
    sample_rate: f64,

    //play_button: Entity,

    header: Entity,
    footer: Entity,
    left_channel_level: Entity,
    right_channel_level: Entity,


    collector: Collector,
    controller: SamplePlayerController,

    random_animation: usize,
    follow_playhead: bool,
    panning: bool,

    scrollbar: Entity,
    navigator: Entity,
    navigator_window: Entity,
    time_axis: Entity,
    extend_selection_left: Entity,
    extend_selection_right: Entity,


    waveform_left: Waveform,
    waveform_right: Waveform,
}

impl AppWidget {
    pub fn new(collector: Collector, controller: SamplePlayerController) -> Self {
        Self {
            perf: PerfGraph::new(),
            prevt: std::time::Instant::now(),

            zoom_level: 3,

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
            cursor: 0,
            select: 0,

            channel_mode: ChannelMode::Left,
            units_mode: UnitsMode::Linear,
            play_state: PlayState::Stopped,

            playhead_label: Entity::default(),

            waveview: Entity::default(),

            is_playing: false,
            should_loop: true,
            panning: false,

            header: Entity::default(),
            footer: Entity::default(),
            left_channel_level: Entity::default(),
            right_channel_level: Entity::default(),
            //play_button: Entity::default(),

            collector,
            controller,

            random_animation: std::usize::MAX,
            follow_playhead: false,

            scrollbar: Entity::default(),
            navigator: Entity::default(),
            navigator_window: Entity::default(),
            time_axis: Entity::default(),
            extend_selection_left: Entity::default(),
            extend_selection_right: Entity::default(),

            waveform_left: Waveform::new(),
            waveform_right: Waveform::new(),
        }
    }
}

impl AppWidget {
    // Draw the audio waveforms
    fn draw_channel(
        &self,
        state: &mut State,
        entity: Entity,
        waveform: &Waveform,
        level: usize,
        start: usize,
        posx: f32,
        posy: f32,
        width: f32,
        height: f32,
        canvas: &mut Canvas<OpenGl>,
    ) {
        let x = posx;
        let y = posy;
        let w = width;
        let h = height;

        //if data.len() > 0 {
        //let audio = &data[self.start as usize..self.end as usize];
        //let audio = data;

        // // Minimum time
        // let start_time = self.start as f32 / self.sample_rate as f32;
        // let end_time = self.end as f32 / self.sample_rate as f32;

        // // Draw grid
        // let first = round_up(start_time as u32, 1);
        // let last = round_up(end_time as u32, 1);

        // for n in (first..last+1) {
        //     let sample = self.sample_rate as f32 * n as f32 - self.start as f32;
        //     let pixel = ((1.0 / self.samples_per_pixel as f32) * sample).round();
        //     let mut path = Path::new();
        //     path.move_to(x + pixel, y + 20.0);
        //     path.line_to(x + pixel, y + 30.0);
        //     let mut paint = Paint::color(femtovg::Color::rgba(90, 90, 90, 255));
        //     paint.set_line_width(1.0);
        //     paint.set_anti_alias(false);
        //     canvas.stroke_path(&mut path, paint);
        // }

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

        //let samples_per_pixel = audio.len() as f32 / w;

        //let mut chunks = audio.chunks(samples_per_pixel as usize);
        //let spp = self.num_of_samples as f32 / w;
        //let mut chunks = audio.chunks(spp as usize);

        //println!("Samples per pixel: {}  {}", self.samples_per_pixel, audio.len() as f32 / w);

        let waveform_data = &waveform.get_data(level);

        //println!("Start: {}", start);

        for pixel in 0..w as usize {
            if start + pixel >= waveform_data.len() {
                break;
            }

            let v_min = to_f32(waveform_data[start + pixel].0);
            let v_max = to_f32(waveform_data[start + pixel].1);
            let v_mean = to_f32(waveform_data[start + pixel].2);

            match self.units_mode {
                UnitsMode::Decibel => {
                    let v_min_db = 1.0 + (20.0 * v_min.abs().log10()).max(-60.0) / 60.0;
                    let v_max_db = 1.0 + (20.0 * v_max.abs().log10()).max(-60.0) / 60.0;

                    let v_mean_db = 1.0 + (20.0 * v_mean.abs().log10()).max(-60.0) / 60.0;

                    let v_min_db = if v_min < 0.0 { -v_min_db } else { v_min_db };

                    let v_max_db = if v_max < 0.0 { -v_max_db } else { v_max_db };

                    let v_mean_db = if v_mean < 0.0 { -v_mean_db } else { v_mean_db };

                    path1.line_to(x + (pixel as f32), y + h / 2.0 - v_min_db * h / 2.0);
                    path1.line_to(x + (pixel as f32), y + h / 2.0 - v_max_db * h / 2.0);

                    path2.move_to(x + (pixel as f32), y + h / 2.0 + v_mean_db * h / 2.0);
                    path2.line_to(x + (pixel as f32), y + h / 2.0 - v_mean_db * h / 2.0);
                }

                UnitsMode::Linear => {
                    path1.line_to(x + (pixel as f32), y + h / 2.0 - v_min * h / 2.0);
                    path1.line_to(x + (pixel as f32), y + h / 2.0 - v_max * h / 2.0);

                    path2.move_to(x + (pixel as f32), y + h / 2.0 + v_mean * h / 2.0);
                    path2.line_to(x + (pixel as f32), y + h / 2.0 - v_mean * h / 2.0);
                }
            }
        }

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
    }
}

impl BuildHandler for AppWidget {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        entity.set_flex_grow(state, 1.0);

        state.focused = entity;
        // TEMP - Animations cause the event loop to run continuously so that the timeline cursor moves smoothly
        let animation_state = AnimationState::new()
            .with_duration(std::time::Duration::from_secs(2000))
            .with_keyframe((0.0, Color::rgb(0, 0, 0)))
            .with_keyframe((1.0, Color::rgb(255, 255, 255)))
            .set_persistent(true);

        self.random_animation = state
            .style
            .border_color
            .insert_animation(animation_state.clone());

        // Header
        //let header = Element::new().build(state, entity, |builder| builder.class("header"));

        self.header = Header::new().build(state, entity, |builder| builder.class("header"));

        self.navigator = Element::new().build(state, entity, |builder| {
            builder.class("navigator").set_height(Length::Pixels(100.0))
        });

        self.navigator_window = Element::new().build(state, self.navigator, |builder| {
            builder
                .class("navigator_window")
                .set_position(Position::Absolute)
                .set_height(Length::Pixels(100.0))
        });

        self.time_axis = Element::new().build(state, entity, |builder| {
            builder.class("time_axis").set_height(Length::Pixels(20.0)).set_left(Length::Pixels(20.0))
        });

        let row = HBox::new().build(state, entity, |builder| builder.set_flex_grow(1.0).set_hoverability(false));

        let amplitude_axis = Element::new().build(state, row, |builder| 
            builder
                .set_width(Length::Pixels(20.0))
                //.set_background_color(Color::green())
                .class("time_axis")
        );

        // Used to add space between header and footer but isn't actually visible
        // TODO - create widget that actually displays a waveform
        self.waveview = Element::new().build(state, row, |builder| {
            builder.class("waveview").set_hoverability(false)
            //.set_visibility(Visibility::Invisible)
        });

        let levels = HBox::new().build(state, row, |builder| builder.set_width(Length::Pixels(50.0)).set_background_color(Color::red()));

        self.left_channel_level = AudioLevelBar::new().build(state, levels, |builder| builder.set_flex_grow(1.0).set_background_color(Color::green()));
        self.right_channel_level = AudioLevelBar::new().build(state, levels, |builder| builder.set_flex_grow(1.0).set_background_color(Color::green()));

        // self.extend_selection_left =
        //     Button::with_label("<").build(state, self.waveview, |builder| {
        //         builder
        //             .set_position(Position::Absolute)
        //             .set_top(Length::Pixels(10.0))
        //             .set_width(Length::Pixels(30.0))
        //             .set_height(Length::Pixels(30.0))
        //             .set_background_color(Color::red())
        //             .set_visibility(Visibility::Invisible)
        //     });

        // self.extend_selection_right =
        //     Button::with_label("<").build(state, self.waveview, |builder| {
        //         builder
        //             .set_position(Position::Absolute)
        //             .set_top(Length::Pixels(10.0))
        //             .set_width(Length::Pixels(30.0))
        //             .set_height(Length::Pixels(30.0))
        //             .set_background_color(Color::red())
        //             .set_visibility(Visibility::Invisible)
        //     });

        // Footer
        self.footer = Footer::new().build(state, entity, |builder| builder.class("footer"));


        // FOOTER



        entity
    }
}

impl EventHandler for AppWidget {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        // Handle window events
        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {
                // When the window width changes, display more or less of the waveform
                WindowEvent::GeometryChanged(geo) => {
                    if geo.width {
                        if event.target == entity {
                            let total_samples =
                                (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as i32;
                            self.end = self.start + (total_samples as usize).min(self.num_of_samples);
                            if let Some(file) = self.controller.file.as_ref() {
                                self.waveform_left.set_num_pixels(
                                    &file.data[0..self.num_of_samples],
                                    state.data.get_width(self.navigator) as usize,
                                );
                                if self.num_of_channels == 2 {
                                    self.waveform_right.set_num_pixels(
                                        &file.data[self.num_of_samples..self.num_of_samples * 2],
                                        state.data.get_width(self.navigator) as usize,
                                    );
                                }
                                
                            }
                        }                        
                    }
                }

                WindowEvent::MouseDown(button) => {
                    // Clicking on the waveform moves the cursor to that position
                    if event.target == entity {
                        if *button == MouseButton::Left {
                            // Move cursor to clicked position
                            let cursor_pos_pixel =
                                state.mouse.left.pos_down.0 - state.data.get_posx(self.waveview);
                            self.cursor = self.start
                                + (self.samples_per_pixel as f32 * cursor_pos_pixel) as usize;
                            self.select = self.cursor;

                            // Update cursor label
                            let time = self.cursor as f32 / self.sample_rate as f32;
                            let time_value: TimeValue = time.into();
                            let time_string = format!("Cursor:  {}", time_value);
                            state.insert_event(Event::new(InfoEvent::SetCursorLabel(time_string)).target(self.footer));

                            // Move extend buttons to cursor
                            self.extend_selection_left
                                .set_left(state, Length::Pixels(cursor_pos_pixel - 40.0));
                            self.extend_selection_right
                                .set_left(state, Length::Pixels(cursor_pos_pixel + 10.0));
                        }
                    }
                    // Clicking on the navigator window allows smooth panning of the waveform
                    if event.target == self.navigator_window && !self.follow_playhead {
                        self.panning = true;
                        state.capture(entity);
                        event.consume();
                    }
                }

                WindowEvent::MouseUp(button) => {
                    if *button == MouseButton::Left {
                        self.panning = false;
                        state.release(entity);
                        //event.consume();
                    }
                }

                // Moving the mouse moves the cursor position
                WindowEvent::MouseMove(x, _) => {
                    if event.target == entity {
                        if self.panning {
                            let width = state.data.get_width(self.navigator);
                            let window_width = state.data.get_width(self.navigator_window);

                            //let mut dx = (state.mouse.left.pos_down.0 - state.data.get_posx(entity));
                            let mut dx = *x - state.data.get_posx(self.waveview);

                            if dx <= window_width / 2.0 {
                                dx = window_width / 2.0;
                            }
                            if dx >= width - window_width / 2.0 {
                                dx = width - window_width / 2.0;
                            }

                            let nx = dx - window_width / 2.0;

                            self.navigator_window.set_left(state, Length::Pixels(nx));

                            self.start = ((nx / width) * self.num_of_samples as f32) as usize;
                            self.end = ((window_width / width) * self.num_of_samples as f32)
                                as usize
                                + self.start;
                        } else {
                            if state.mouse.left.pressed == entity
                                && state.mouse.left.state == MouseButtonState::Pressed
                            {
                                let start_pos =
                                    state.mouse.left.pos_down.0 - state.data.get_posx(self.waveview);
                                let end_pos = *x - state.data.get_posx(self.waveview);

                                if start_pos > end_pos {
                                    self.cursor = self.start
                                        + (self.samples_per_pixel as f32 * end_pos) as usize;
                                    self.select = self.start
                                        + (self.samples_per_pixel as f32 * start_pos) as usize;
                                } else if end_pos > start_pos {
                                    self.cursor = self.start
                                        + (self.samples_per_pixel as f32 * start_pos) as usize;
                                    self.select = self.start
                                        + (self.samples_per_pixel as f32 * end_pos) as usize;
                                }

                                let time = self.cursor as f32 / self.sample_rate as f32;
                                let time_value: TimeValue = time.into();
                                let time_string = format!("Cursor:  {}", time_value);
                                state.insert_event(Event::new(InfoEvent::SetCursorLabel(time_string)).target(self.footer));
                                //self.cursor_label.set_text(state, &time_string);

                                let time = self.select as f32 / self.sample_rate as f32;
                                let time_value: TimeValue = time.into();
                                let time_string = format!("Select End:  {}", time_value);
                                //self.select_label.set_text(state, &time_string);
                                state.insert_event(Event::new(InfoEvent::SetSelectLabel(time_string)).target(self.footer));

                                // if (end_pos - start_pos).abs() > 2.0 {
                                //     self.select =  self.start + (self.samples_per_pixel as f32 * select_end_pos) as usize;
                                // }

                                state.insert_event(Event::new(WindowEvent::Redraw));
                            }

                            if self.num_of_samples > 0 {
                                self.zoom_pos_pixel = *x - state.data.get_posx(entity);

                                self.zoom_pos = self.start
                                    + (self.samples_per_pixel as f32 * self.zoom_pos_pixel)
                                        as usize;

                                if self.zoom_pos >= self.num_of_samples {
                                    self.zoom_pos = self.num_of_samples.saturating_sub(1);
                                }
                            }
                        }
                    }
                }

                // Scrolling the mouse will pan the waveform, scrolling with ctrl will zoom the waveform at the cursor
                WindowEvent::MouseScroll(_, y) => {
                    if *y > 0.0 {
                        if state.modifiers.ctrl {
                            // ZOOM IN
                            if self.zoom_level != SAMPLES_PER_PIXEL.len() - 1 {
                                self.zoom_level += 1;
                            } else {
                                return;
                            }

                            state.insert_event(
                                Event::new(AppEvent::SetZoomLevel(
                                    self.zoom_level,
                                    ZoomMode::Mouse,
                                ))
                                .target(entity),
                            );
                        } else {
                            // PAN
                            let samples_to_start = self.start;

                            let new_start;
                            let new_end;

                            if samples_to_start < (self.samples_per_pixel as f32 * 30.0) as usize {
                                new_start = self.start - samples_to_start;
                                new_end = self.end - samples_to_start;
                            } else {
                                new_start =
                                    self.start - (self.samples_per_pixel as f32 * 30.0) as usize;
                                new_end =
                                    self.end - (self.samples_per_pixel as f32 * 30.0) as usize;
                            }

                            self.end = new_end.min(self.num_of_samples.saturating_sub(1));
                            self.start = new_start.max(0).min(self.end);
                        }

                        state.insert_event(Event::new(WindowEvent::Redraw));
                        event.consume();
                    } else if *y < 0.0 {
                        if state.modifiers.ctrl {
                            // ZOOM OUT
                            if self.zoom_level != 0 {
                                self.zoom_level -= 1;
                            } else {
                                return;
                            }

                            state.insert_event(
                                Event::new(AppEvent::SetZoomLevel(
                                    self.zoom_level,
                                    ZoomMode::Mouse,
                                ))
                                .target(entity),
                            );
                        } else {
                            //PAN

                            let samples_to_end = self.num_of_samples.saturating_sub(1 + self.end);

                            let new_start;
                            let new_end;

                            if samples_to_end < (self.samples_per_pixel as f32 * 30.0) as usize {
                                new_start = self.start + samples_to_end;
                                new_end = self.end + samples_to_end;
                            } else {
                                new_start =
                                    self.start + (self.samples_per_pixel as f32 * 30.0) as usize;
                                //let sample_end = self.start + state.data.get_width(self.waveview) * self.samples_per_pixel;

                                new_end =
                                    self.end + (self.samples_per_pixel as f32 * 30.0) as usize;
                                // if samle_end < new_end {
                                //     new_end
                                // }
                            }

                            self.start = new_start.max(0).min(self.end);
                            self.end = new_end.min(self.num_of_samples.saturating_sub(1));
                        }

                        state.insert_event(Event::new(WindowEvent::Redraw));
                        event.consume();
                    }
                }

                WindowEvent::KeyDown(code, key) => {
                    //println!("Key: {:?} {:?}", code, key);
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

                        _ => {}
                    }

                    match key {
                        Some(Key::ArrowLeft) => {
                            //println!("Do This");
                            if self.playhead > 0 {
                                let current_time = self.playhead as f64 / self.sample_rate;
                                let new_time = (current_time - 1.0).max(0.0);
                                let cursor_time = self.cursor as f64 / self.sample_rate;
                                if self.is_playing {
                                    if new_time <= cursor_time {
                                        self.controller.seek(cursor_time);
                                    } else {
                                        self.controller.seek(new_time);
                                    }
                                } else {
                                    self.playhead -= self.samples_per_pixel;
                                    //println!("playhead: {}", self.playhead);
                                    let current_time =
                                        (self.playhead as f64 / self.sample_rate).max(0.0);
                                    if current_time <= cursor_time {
                                        self.controller.seek(cursor_time);
                                    } else {
                                        self.controller.seek(current_time);
                                    }
                                }

                                let time = self.playhead as f32 / self.sample_rate as f32;
                                let time_value: TimeValue = time.into();
                                let time_string = format!("{}", time_value);
                                self.playhead_label.set_text(state, &time_string);

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
                                let current_time =
                                    (self.playhead as f64 / self.sample_rate).max(0.0);
                                self.controller.seek(current_time);
                            }

                            let time = self.playhead as f32 / self.sample_rate as f32;
                            let time_value: TimeValue = time.into();
                            let time_string = format!("{}", time_value);
                            self.playhead_label.set_text(state, &time_string);

                            state.insert_event(Event::new(WindowEvent::Redraw));
                        }

                        Some(Key::Home) => {
                            state.insert_event(Event::new(AppEvent::SeekLeft).target(entity));
                        }

                        Some(Key::End) => {
                            state.insert_event(Event::new(AppEvent::SeekRight).target(entity));
                        }

                        _ => {}
                    }
                }

                WindowEvent::MouseOver => {
                    //println!("Mouse Over: {}", event.target);
                    //println!("Tooltip: {}", state.style.tooltip.get(event.target).cloned().unwrap_or_default());
                    state.insert_event(Event::new(InfoEvent::SetTooltip(state.style.tooltip.get(event.target).cloned().unwrap_or_default())).target(self.footer));
                    event.consume();
                }

                _ => {}
            }
        }

        // Handle application events
        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {
                // Load an audio file specified on the command line
                AppEvent::LoadAudioFile(file_path) => {
        
                    // self.controller.load_file(file_path);

                    // if let Some(file) = self.controller.file.as_ref() {
                    //     self.num_of_channels = file.num_channels;
                    //     self.sample_rate = file.sample_rate;
                    //     self.num_of_samples = file.num_samples;
                    //     println!("Length: {} ", self.num_of_samples);
                    // }

                    state.insert_event(
                        Event::new(AppEvent::SetZoomLevel(3, ZoomMode::Cursor)).target(entity),
                    );
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
                            
                            self.controller
                                .load_file(file_path.as_os_str().to_str().unwrap());
                            self.controller.seek(0.0);
                            self.is_playing = false;

                            if let Some(file) = self.controller.file.as_ref() {
                                self.num_of_channels = file.num_channels;
                                self.sample_rate = file.sample_rate;
                                self.num_of_samples = file.num_samples;
                                println!("Length: {} ", file.num_samples);

                                self.waveform_left.load(
                                    &file.data[0..self.num_of_samples],
                                    state.data.get_width(self.navigator) as usize,
                                );

                                if self.num_of_channels == 2 {
                                    self.waveform_right.load(
                                        &file.data[self.num_of_samples..self.num_of_samples * 2],
                                        state.data.get_width(self.navigator) as usize,
                                    );                                    
                                }

                            }

                            if let Some(file_name) = file_path.as_path().file_name().and_then(|s| s.to_str()) {
                                state.insert_event(Event::new(InfoEvent::SetFileNameLabel(file_name.to_string())).target(self.footer));
                            }

                            

                            state.insert_event(
                                Event::new(AppEvent::SetZoomLevel(3, ZoomMode::Cursor))
                                    .target(entity),
                            );
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
                AppEvent::SetZoomLevel(val, zoom_mode) => {
                    let (zoom, zoom_pos) = if *zoom_mode == ZoomMode::Mouse {
                        (self.zoom_pos, self.zoom_pos_pixel)
                    } else if self.select != self.cursor {
                        let zoom = if self.select < self.cursor {
                            self.select + (self.cursor - self.select) / 2
                        } else {
                            self.cursor + (self.select - self.cursor) / 2
                        };

                        (
                            zoom,
                            (zoom - self.start) as f32 / self.samples_per_pixel as f32,
                        )
                    } else {
                        (
                            self.cursor,
                            (self.cursor - self.start) as f32 / self.samples_per_pixel as f32,
                        )
                    };

                    self.zoom_level = *val;
                    self.samples_per_pixel = SAMPLES_PER_PIXEL[self.zoom_level];

                    let total_samples =
                        (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as i32;

                    let mut new_start = 0;
                    let mut new_end = total_samples as usize;

                    let offset = zoom as i32 - (zoom_pos * self.samples_per_pixel as f32) as i32;

                    if offset > 0 {
                        new_start = offset as usize;
                        new_end = total_samples as usize + offset as usize;
                    }

                    self.end = new_end.min(self.num_of_samples.saturating_sub(1));
                    self.start = new_start.max(0).min(self.end);

                    state.insert_event(Event::new(AppEvent::SetZoomLevel(*val, zoom_mode.clone())).target(self.footer).propagate(Propagation::Direct));

                   
                    // Manually trigger a redraw
                    state.insert_event(Event::new(WindowEvent::Redraw));
                }

                AppEvent::IncZoom => {
                    if self.zoom_level != SAMPLES_PER_PIXEL.len() - 1 {
                        self.zoom_level += 1;

                        state.insert_event(
                            Event::new(AppEvent::SetZoomLevel(self.zoom_level, ZoomMode::Cursor))
                                .target(entity),
                        );
                    } 
                }

                AppEvent::DecZoom => {
                    if self.zoom_level != 0 {
                        self.zoom_level -= 1;
                        state.insert_event(
                            Event::new(AppEvent::SetZoomLevel(self.zoom_level, ZoomMode::Cursor))
                                .target(entity),
                        );
                    }
                }

                // Initiate playback
                AppEvent::Play => {
                    // Forward event to the header to update the play button
                    state.insert_event(Event::new(AppEvent::Play).target(self.header).propagate(Propagation::Direct));

                    // If stopped, play from cursor
                    if self.play_state == PlayState::Stopped {
                        if self.cursor != self.start {
                            let cursor_time = self.cursor as f64 / self.sample_rate;
                            self.controller.seek(cursor_time);
                        }
                    }

                    self.controller.play();

                    state
                        .style
                        .border_color
                        .play_animation(entity, self.random_animation);

                    self.is_playing = true;
                    self.play_state = PlayState::Playing;
                }

                // Pause playback
                AppEvent::Pause => {
                    self.play_state = PlayState::Paused;
                    // Forward event to the header to update the play button
                    state.insert_event(Event::new(AppEvent::Pause).target(self.header).propagate(Propagation::Direct));
                    self.controller.stop();
                    self.is_playing = false;
                }

                // Stop playback
                AppEvent::Stop => {
                    self.play_state = PlayState::Stopped;

                    self.controller.stop();

                    if self.cursor != self.start {
                        let cursor_time = self.cursor as f64 / self.sample_rate;
                        self.controller.seek(cursor_time);
                        self.playhead = self.cursor;
                    } else {
                        self.controller.seek(0.0);
                        self.playhead = 0;
                    }

                    // Forward event to the header to update the play button
                    state.insert_event(Event::new(AppEvent::Stop).target(self.header).propagate(Propagation::Direct));
                    self.is_playing = false;

                    if self.follow_playhead {
                        let playhead_pos_pixel = state.data.get_width(self.waveview) / 2.0;
                        let offset = self.playhead as i32
                            - (playhead_pos_pixel * self.samples_per_pixel as f32) as i32;
                        let total_samples =
                            (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as usize;

                        let mut new_start = 0;
                        let mut new_end = total_samples;

                        if offset > 0 {
                            new_start = offset as usize;
                            new_end = total_samples as usize + offset as usize;
                        }

                        self.end = new_end.min(self.num_of_samples.saturating_sub(1));
                        self.start = new_start.max(0).min(self.end);
                    }
                }

                // Move playhead to start
                AppEvent::SeekLeft => {
                    self.controller.seek(0.0);
                    let total_samples =
                        (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as i32;
                    self.start = 0;
                    self.end = (total_samples as usize).min(self.num_of_samples);
                    self.start = self.start.max(0);
                    self.end = self.end.min(self.num_of_samples.saturating_sub(1));
                    self.cursor = 0;
                    self.select = 0;

                    // let time = self.playhead as f32 / self.sample_rate as f32;
                    // let time_value: TimeValue = time.into();
                    // let time_string = format!("{}", time_value);
                    // self.playhead_label.set_text(state, &time_string);
                }

                // Move playhead to end
                // TODO
                AppEvent::SeekRight => {
                    self.controller.stop();
                    let end_time = self.num_of_samples as f64 / self.sample_rate;
                    self.controller.seek(end_time);
                    state.insert_event(Event::new(AppEvent::Stop).target(self.header).propagate(Propagation::Direct));
                }

                

                AppEvent::FollowPlayhead(val) => {
                    self.follow_playhead = *val;

                    let playhead_pos_pixel = state.data.get_width(self.waveview) / 2.0;
                    let offset = self.playhead as i32
                        - (playhead_pos_pixel * self.samples_per_pixel as f32) as i32;
                    let total_samples =
                        (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as usize;

                    let mut new_start = 0;
                    let mut new_end = total_samples;

                    if offset > 0 {
                        new_start = offset as usize;
                        new_end = total_samples as usize + offset as usize;
                    }

                    self.end = new_end.min(self.num_of_samples.saturating_sub(1));
                    self.start = new_start.max(0).min(self.end);
                }

                AppEvent::Loop(val) => {
                    self.should_loop = *val;
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

        self.collector.collect();

        let now = std::time::Instant::now();
        let dt = (now - self.prevt).as_secs_f32();
        self.prevt = now;

        self.perf.update(dt);

        let x = state.data.get_posx(self.waveview);
        let y = state.data.get_posy(self.waveview);
        let h = state.data.get_height(self.waveview);
        let w = state.data.get_width(self.waveview);

        let navigator_posx = state.data.get_posx(self.navigator);
        let navigator_posy = state.data.get_posy(self.navigator);
        let navigator_width = state.data.get_width(self.navigator);
        let navigator_height = state.data.get_height(self.navigator);

        let mut path = Path::new();
        path.rect(
            navigator_posx,
            navigator_posy,
            navigator_width,
            navigator_height,
        );
        canvas.fill_path(
            &mut path,
            Paint::color(femtovg::Color::rgba(30, 30, 30, 255)),
        );

        let mut path = Path::new();
        path.rect(x, y, w, h);
        canvas.fill_path(
            &mut path,
            Paint::color(femtovg::Color::rgba(30, 30, 30, 255)),
        );

        let cursor_pos = if self.cursor > self.start && self.cursor < self.end {
            (self.cursor - self.start) as f32 / self.samples_per_pixel as f32
        } else if self.cursor > self.end {
            x + w
        } else {
            x
        };

        let select_pos = if self.select > self.start && self.select < self.end {
            (self.select - self.start) as f32 / self.samples_per_pixel as f32
        } else if self.select > self.end {
            x + w
        } else {
            x
        };

        // Draw selection
        let mut path = Path::new();
        if self.cursor < self.select {
            path.rect(x + cursor_pos, y, select_pos - cursor_pos, h);
        } else if self.select < self.cursor {
            path.rect(x + select_pos, y, cursor_pos - select_pos, h);
        }
        canvas.fill_path(
            &mut path,
            Paint::color(femtovg::Color::rgba(60, 60, 60, 150)),
        );

        if let Some(file) = self.controller.file.as_ref() {
            let time_axis_posx = state.data.get_posx(self.time_axis);
            let time_axis_posy = state.data.get_posy(self.time_axis);
            let time_axis_width = state.data.get_width(self.time_axis);
            let time_axis_height = state.data.get_height(self.time_axis);

            // Minimum time
            let start_time = self.start as f32 / self.sample_rate as f32;
            let end_time = self.end as f32 / self.sample_rate as f32;

            // Draw grid
            let first = round_up(start_time as u32, 1);
            let last = round_up(end_time as u32, 1);

            for n in (first..last + 1) {
                let sample = self.sample_rate as f32 * n as f32 - self.start as f32;
                let pixel = ((1.0 / self.samples_per_pixel as f32) * sample).round();
                let mut path = Path::new();
                path.move_to(time_axis_posx + pixel, time_axis_posy);
                path.line_to(time_axis_posx + pixel, time_axis_posy + time_axis_height);
                let mut paint = Paint::color(femtovg::Color::rgba(90, 90, 90, 255));
                paint.set_line_width(1.0);
                paint.set_anti_alias(false);
                canvas.stroke_path(&mut path, paint);
            }

            let start = round_up(self.start as u32, self.samples_per_pixel as u32) as usize
                / self.samples_per_pixel;

            match self.channel_mode {
                ChannelMode::Left => {
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_left,
                        SAMPLES_PER_PIXEL.len(),
                        0,
                        navigator_posx,
                        navigator_posy,
                        navigator_width,
                        navigator_height,
                        canvas,
                    );
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_left,
                        self.zoom_level,
                        start,
                        x,
                        y,
                        w,
                        h,
                        canvas,
                    );
                }

                ChannelMode::Right => {
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_right,
                        SAMPLES_PER_PIXEL.len(),
                        0,
                        navigator_posx,
                        navigator_posy,
                        navigator_width,
                        navigator_height,
                        canvas,
                    );
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_right,
                        self.zoom_level,
                        start,
                        x,
                        y,
                        w,
                        h,
                        canvas,
                    );
                }

                ChannelMode::Both => {
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_left,
                        SAMPLES_PER_PIXEL.len(),
                        0,
                        navigator_posx,
                        navigator_posy,
                        navigator_width,
                        navigator_height / 2.0,
                        canvas,
                    );
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_right,
                        SAMPLES_PER_PIXEL.len(),
                        0,
                        navigator_posx,
                        navigator_posy + navigator_height / 2.0,
                        navigator_width,
                        navigator_height / 2.0,
                        canvas,
                    );
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_left,
                        self.zoom_level,
                        start,
                        x,
                        y,
                        w,
                        h / 2.0,
                        canvas,
                    );
                    self.draw_channel(
                        state,
                        entity,
                        &self.waveform_right,
                        self.zoom_level,
                        start,
                        x,
                        y + h / 2.0,
                        w,
                        h / 2.0,
                        canvas,
                    );
                }
            }
        }

        // Draw Navigator Window
        let window_posx = (self.start as f32 / self.num_of_samples as f32) * navigator_width;
        let window_width =
            ((self.end - self.start) as f32 / self.num_of_samples as f32) * navigator_width;

        if !self.panning {
            self.navigator_window
                .set_left(state, Length::Pixels(window_posx))
                .set_width(state, Length::Pixels(window_width));
        }

        // let mut path = Path::new();
        // path.rect(window_posx, navigator_posy, window_width, navigator_height);
        // canvas.fill_path(
        //     &mut path,
        //     Paint::color(femtovg::Color::rgba(120, 120, 120, 100)),
        // );
        // canvas.stroke_path(&mut path, Paint::color(femtovg::Color::rgba(200, 200, 200, 100)));

        // Draw playhead
        let playhead = self.controller.playhead() as f64;

        let pixels_per_sample = 1.0 / self.samples_per_pixel as f32;
        let playheadx = x + pixels_per_sample * (playhead as f32 - self.start as f32);

        let mut path = Path::new();
        if self.follow_playhead && self.start > 0 {
            path.move_to(x + w / 2.0, y);
            path.line_to(x + w / 2.0, y + h);
        } else {
            path.move_to(playheadx.floor(), y);
            path.line_to(playheadx.floor(), y + h);
        }
        let mut paint = Paint::color(femtovg::Color::rgba(50, 200, 50, 255));
        paint.set_line_width(1.0);
        paint.set_anti_alias(false);
        canvas.stroke_path(&mut path, paint);

        // Draw navigator playhead
        let playheadx =
            navigator_posx + (navigator_width / self.num_of_samples as f32) * playhead as f32;

        let mut path = Path::new();
        path.move_to(playheadx.floor(), navigator_posy);
        path.line_to(playheadx.floor(), navigator_posy + navigator_height);
        let mut paint = Paint::color(femtovg::Color::rgba(50, 200, 50, 255));
        paint.set_line_width(1.0);
        paint.set_anti_alias(false);
        canvas.stroke_path(&mut path, paint);

        // Draw cursor
        if self.cursor > self.start && self.cursor < self.end {
            let mut path = Path::new();
            path.move_to((x + cursor_pos).floor(), y);
            path.line_to((x + cursor_pos).floor(), y + h);
            let mut paint = Paint::color(femtovg::Color::rgba(255, 50, 50, 255));
            paint.set_line_width(1.0);
            paint.set_anti_alias(false);
            canvas.stroke_path(&mut path, paint);
        }

        let cursorx =
            navigator_posx + (navigator_width / self.num_of_samples as f32) * self.cursor as f32;

        let mut path = Path::new();
        path.move_to((navigator_posx + cursorx).round(), navigator_posy);
        path.line_to(
            (navigator_posx + cursorx).round(),
            navigator_posy + navigator_height,
        );
        let mut paint = Paint::color(femtovg::Color::rgba(255, 50, 50, 255));
        paint.set_line_width(1.0);
        paint.set_anti_alias(false);
        canvas.stroke_path(&mut path, paint);

        // Reset the playhead when it reaches the end of the file
        let playhead = self.controller.playhead() as f64;
        self.playhead = playhead as usize;

        let loop_end = if self.select != self.cursor {
            self.select
        } else {
            self.num_of_samples
        };

        // if self.playhead > loop_end {
        //     if self.should_loop {
        //         let cursor_time = self.cursor as f64 / self.sample_rate;
        //         self.controller.seek(cursor_time);
        //         self.playhead = self.cursor;
        //     } else {
        //         state.insert_event(Event::new(AppEvent::Stop).target(entity));
        //     }
        // }

        if self.should_loop {
            if self.playhead > loop_end {
                let cursor_time = self.cursor as f64 / self.sample_rate;
                self.controller.seek(cursor_time);
                self.playhead = self.cursor;
            }
        } else {
            if self.playhead > self.num_of_samples {
                state.insert_event(Event::new(AppEvent::Stop).target(entity));
            }
        }

        // if let Some(file) = self.controller.file.as_ref() {
        //     if playhead as usize > file.num_samples {
        //         state.insert_event(Event::new(AppEvent::Stop).target(entity));
        //     }
        // }

        // Update the playhead time display
        if self.is_playing {
            let time = self.playhead as f32 / self.sample_rate as f32;
            let time_value: TimeValue = time.into();
            let time_string = format!("{}", time_value);
            //self.playhead_label.set_text(state, &time_string);
            state.insert_event(Event::new(InfoEvent::SetTimeLabel(time_string)).target(self.header));
            if let Some(file) = &self.controller.file {
                //println!("Playehead: {}", round_up(self.playhead as u32, 441));
                let start = round_up(self.playhead as u32, 441) as usize;
                //let mag = file.data[start..start+441].iter().sum::<f32>() / 441.0;
                let v_max = file.data[start..start+441]
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();
                //println!("Mag: {}", mag);
                state.insert_event(Event::new(AudioLevelEvent::SetLevel(*v_max)).target(self.left_channel_level).propagate(Propagation::Direct));
            }
        }

        // Move the waveform if following the playhead
        if self.follow_playhead {
            //println!("playhead: {} {}", self.playhead, self.samples_per_pixel as usize);
            //if self.playhead % self.samples_per_pixel == 0 {
            //println!("playhead: {}", playhead);
            let playhead_pos_pixel = w / 2.0;
            let offset =
                self.playhead as i32 - (playhead_pos_pixel * self.samples_per_pixel as f32) as i32;
            let total_samples =
                (state.data.get_width(self.waveview) * self.samples_per_pixel as f32) as usize;

            let mut new_start = 0;
            let mut new_end = total_samples;

            if offset > 0 {
                new_start = offset as usize;
                new_end = total_samples as usize + offset as usize;
            }

            self.end = new_end.min(self.num_of_samples.saturating_sub(1));
            self.start = new_start.max(0).min(self.end);
            //}
        }
        self.perf.render(canvas, 0.0, 200.0);
        // Dirty hack to reduce cpu usage until I do something better in tuix
        //std::thread::sleep(std::time::Duration::from_millis(5));
    }
}




struct PerfGraph {
    history_count: usize,
    values: Vec<f32>,
    head: usize,
}

impl PerfGraph {
    fn new() -> Self {
        Self {
            history_count: 100,
            values: vec![0.0; 100],
            head: Default::default(),
        }
    }

    fn update(&mut self, frame_time: f32) {
        self.head = (self.head + 1) % self.history_count;
        self.values[self.head] = frame_time;
    }

    fn get_average(&self) -> f32 {
        self.values.iter().map(|v| *v).sum::<f32>() / self.history_count as f32
    }

    fn render<T: femtovg::Renderer>(&self, canvas: &mut Canvas<T>, x: f32, y: f32) {
        let avg = self.get_average();

        let w = 200.0;
        let h = 35.0;

        let mut path = Path::new();
        path.rect(x, y, w, h);
        canvas.fill_path(&mut path, Paint::color(femtovg::Color::rgba(0, 0, 0, 128)));

        let mut path = Path::new();
        path.move_to(x, y + h);

        for i in 0..self.history_count {
            let mut v = 1.0 / (0.00001 + self.values[(self.head + i) % self.history_count]);
            if v > 80.0 {
                v = 80.0;
            }
            let vx = x + (i as f32 / (self.history_count.saturating_sub(1)) as f32) * w;
            let vy = y + h - ((v / 80.0) * h);
            path.line_to(vx, vy);
        }

        path.line_to(x + w, y + h);
        canvas.fill_path(
            &mut path,
            Paint::color(femtovg::Color::rgba(255, 192, 0, 128)),
        );

        let mut text_paint = Paint::color(femtovg::Color::rgba(240, 240, 240, 255));
        text_paint.set_font_size(12.0);
        let _ = canvas.fill_text(x + 5.0, y + 13.0, "Frame time", text_paint);

        let mut text_paint = Paint::color(femtovg::Color::rgba(240, 240, 240, 255));
        text_paint.set_font_size(14.0);
        text_paint.set_text_align(femtovg::Align::Right);
        text_paint.set_text_baseline(femtovg::Baseline::Top);
        let _ = canvas.fill_text(x + w - 5.0, y, &format!("{:.2} FPS", 1.0 / avg), text_paint);

        let mut text_paint = Paint::color(femtovg::Color::rgba(240, 240, 240, 200));
        text_paint.set_font_size(12.0);
        text_paint.set_text_align(femtovg::Align::Right);
        text_paint.set_text_baseline(femtovg::Baseline::Alphabetic);
        let _ = canvas.fill_text(
            x + w - 5.0,
            y + h - 5.0,
            &format!("{:.2} ms", avg * 1000.0),
            text_paint,
        );
    }
}
