
extern crate tuix;

// use std::{fs::read, path::Path};

const ICON_TO_START: &str = "\u{23ee}";
const ICON_PLAY: &str = "\u{25b6}";
const ICON_STOP: &str = "\u{25a0}";
const ICON_TO_END: &str = "\u{23ed}";
const ICON_PLUS: &str = "\u{2b}";
const ICON_MINUS: &str = "\u{2d}";

use tuix::*;

use native_dialog::{FileDialog};

use std::{cmp::Ordering, println};

use dasp_sample::{Sample, I24};

use femtovg::{
    //CompositeOperation,
    renderer::OpenGl,
    Align,
    Baseline,
    Canvas,
    FillRule,
    FontId,
    ImageFlags,
    ImageId,
    LineCap,
    LineJoin,
    Paint,
    Path,
    Renderer,
    Solidity,
};

pub struct TimeValue(pub f32);

impl std::fmt::Display for TimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.abs() < 1.0 {
            write!(f, "{:.0} ms", self.0 * 1000.0)
        } else if self.0.abs() >= 1.0 && self.0.abs() < 60.0 {
            write!(f, "{:.3} s", self.0)
        } else if self.0.abs() >= 60.0 && self.0.abs() < 3600.0 {
            write!(f, "{:.3} mins", self.0 / 60.0)
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

fn main() {


    

    let app = Application::new(|win_desc, state, window| {

        
        


        state.insert_stylesheet("src/theme.css").expect("Failed to load stylesheet");

        window.set_background_color(state, Color::rgb(40,40,40));
        
        let app_widget = AppWidget::new().build(state, window, |builder| builder.class("app"));

        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            state.insert_event(Event::new(AppEvent::LoadAudioFile(args[1].clone())).target(app_widget));
        }

        win_desc.with_title("Waveform Viewer").with_inner_size(1000, 600)
    });

    app.run();
}


const ZOOM_LEVELS: [f32; 15] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 50.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0];

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

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    OpenFileDialog,
    LoadAudioFile(String),
    SwicthChannel(ChannelMode),
    SwitchUnits(UnitsMode),
}


#[derive(Debug)]
pub struct AppWidget {
    left_channel: Vec<f32>,
    right_channel: Vec<f32>,
    zoom_level: usize,
    scroll_pos: usize,

    samples_per_pixel: f32,

    zoom_pos_pixel: f32,

    start: i32,
    end: i32,
    zoom_pos: i32,

    channel_mode: ChannelMode,
    units_mode: UnitsMode,

    time_label: Entity,
    value_label: Entity,

    waveview: Entity,
}

impl AppWidget {
    pub fn new() -> Self {
        Self {
            left_channel: Default::default(),
            right_channel: Default::default(),
            zoom_level: 2,
            scroll_pos: 0,

            samples_per_pixel: 220.0,

            zoom_pos_pixel: 0.0,

            start: 0,
            end: 0,
            zoom_pos: 0,

            channel_mode: ChannelMode::Left,
            units_mode: UnitsMode::Linear,

            time_label: Entity::null(),
            value_label: Entity::null(),
            waveview: Entity::null(),
            
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

        self.left_channel = vec![0.0; buffer_size/2];
        self.right_channel = vec![0.0; buffer_size/2];


        
        for (interleaved_samples, (left, right)) in audio.chunks(2).zip(self.left_channel.iter_mut().zip(self.right_channel.iter_mut())) {
            *left = interleaved_samples[0];
            *right = interleaved_samples[1];
        }

        Ok(())
    }

    // Draw the audio waveforms
    fn draw_channel(&self, state: &mut State, entity: Entity, data: &[f32], posy: f32, height: f32, canvas: &mut Canvas<OpenGl>) {
        let x = state.transform.get_posx(self.waveview);
        let y = posy;
        let w = state.transform.get_width(self.waveview);
        let h = height;
        
        if data.len() > 0 {
            let audio = &data[self.start as usize..self.end as usize];


            let mut path = Path::new();
            path.rect(x, y, w, h);
            canvas.fill_path(&mut path, Paint::color(femtovg::Color::rgba(40, 40, 40, 255)));
    
    
            let mut path = Path::new();

            path.move_to(x, y + h/2.0);
    
            if self.samples_per_pixel < 1.0 {
                for pixel in 0..w as u32 {
                    let pixels_per_sample = (1.0 / self.samples_per_pixel) as u32;
                    
                    if pixel  % pixels_per_sample == 0 {

                        let sample = self.start + (self.samples_per_pixel * pixel as f32).floor() as i32;
                        path.move_to(x + (pixel as f32), y + h/2.0);
                        path.line_to(x + (pixel as f32), y + h/2.0 - data[sample as usize]* h/2.0);
                    }
                }
            } else {
    
                let mut chunks = audio.chunks(self.samples_per_pixel.round() as usize);
    
                for chunk in 0..w as u32 {
        
                    if let Some(c) = chunks.next() {
                        let v_min = *c.iter().min_by(|a,b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap();
                        let v_max = *c.iter().max_by(|a,b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap();


                        match self.units_mode {
                            UnitsMode::Decibel => {
                                let mut v_min_db = 1.0 + (20.0 * v_min.abs().log10()).max(-60.0) / 60.0;
                                let mut v_max_db = 1.0 + (20.0 * v_max.abs().log10()).max(-60.0) / 60.0;
    
                                let v_min_db = if v_min < 0.0 {
                                    -v_min_db
                                } else {
                                    v_min_db
                                };
    
                                let v_max_db = if v_max < 0.0 {
                                    -v_max_db
                                } else {
                                    v_max_db
                                };
    
                                
                                path.line_to(x + (chunk as f32), y + h/2.0 - v_min_db * h/2.0);
                                path.line_to(x + (chunk as f32), y + h/2.0 - v_max_db * h/2.0);
                            }

                            UnitsMode::Linear => {
                                path.line_to(x + (chunk as f32), y + h/2.0 - v_min * h/2.0);
                                path.line_to(x + (chunk as f32), y + h/2.0 - v_max * h/2.0);
                            }
                        }
                    }
                }
            }
    
    
            let mut paint = Paint::color(femtovg::Color::rgba(50, 50, 255, 255));
            paint.set_line_width(1.0);
            paint.set_anti_alias(false);
            canvas.stroke_path(&mut path, paint);

            // Draw cursor
            let mut path = Path::new();
            path.move_to(x + self.zoom_pos_pixel, y);
            path.line_to(x + self.zoom_pos_pixel, y + h);
            paint.set_line_width(1.0);
            paint.set_anti_alias(false);
            canvas.fill_path(&mut path, Paint::color(femtovg::Color::rgba(255, 50, 50, 255)));

        }
    }
}

impl BuildHandler for AppWidget {
    type  Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {

        entity.set_flex_grow(state, 1.0);


        let header = Element::new().build(state, entity, |builder| builder.class("header"));
        self.waveview = Element::new().build(state, entity, |builder| builder.class("waveview").set_visibility(Visibility::Invisible));
        let footer = Element::new().build(state, entity, |builder| builder.class("footer"));

        // Open file button
        Button::new().on_release(Event::new(AppEvent::OpenFileDialog)).build(state, header, |builder| {
            builder.set_text("Open").set_margin(Length::Pixels(10.0)).class("open")
        });


        let transport = Element::new().build(state, header, |builder| builder.class("checklist"));

        // To start button
        Button::new().build(state, transport, |builder| {
            builder.set_text(ICON_TO_START).set_font("Icons".to_string()).class("first")
        });

        // Play button
        let play =  Button::new().build(state, transport, |builder| {
            builder.set_text(ICON_PLAY).set_font("Icons".to_string()).class("play")
        });

        // Stop button
        Button::new().build(state, transport, |builder| {
            builder.set_text(ICON_STOP).set_font("Icons".to_string())
        });

        // To end button
        Button::new().build(state, transport, |builder| {
            builder.set_text(ICON_TO_END).set_font("Icons".to_string()).class("last")
        });

        // Channels selector
        let channels = CheckList::new().build(state, header, |builder| builder.class("checklist"));

        let left = Button::new()
            .on_press(Event::new(AppEvent::SwicthChannel(ChannelMode::Left)).target(entity))
            .build(state, channels, |builder| {
            builder.set_text("L").class("first")
        });

        left.set_checked(state, true);

        Button::new()
        .on_press(Event::new(AppEvent::SwicthChannel(ChannelMode::Right)).target(entity))
        .build(state, channels, |builder| {
            builder.set_text("R")
        });

        Button::new().on_press(Event::new(AppEvent::SwicthChannel(ChannelMode::Both)).target(entity)).build(state, channels, |builder| {
            builder.set_text("L + R").class("last").set_width(Length::Pixels(60.0))
        });

        self.time_label = Label::new("Time: -").build(state, header, |builder| builder.set_margin(Length::Pixels(10.0)));
        self.value_label = Label::new("Value: -").build(state, header, |builder| builder.set_margin(Length::Pixels(10.0)));

        let units = CheckList::new().build(state, header, |builder| builder.class("checklist"));

        let linear = Button::new().on_press(Event::new(AppEvent::SwitchUnits(UnitsMode::Linear)).target(entity)).build(state, units, |builder| {
            builder.set_text("Mag").class("first")
        });

        linear.set_checked(state, true);

        Button::new().on_press(Event::new(AppEvent::SwitchUnits(UnitsMode::Decibel)).target(entity)).build(state, units, |builder| {
            builder.set_text("dB").class("last")
        });

        let zoom_controls = Element::new().build(state, footer, |builder| builder.class("zoom_controls"));
        Button::with_label(ICON_MINUS).build(state, zoom_controls, |builder| builder.set_font("Icons".to_string()).class("zoom").class("first"));
        Label::new("100%").build(state, zoom_controls, |builder| builder.class("zoom"));
        Button::with_label(ICON_PLUS).build(state, zoom_controls, |builder| builder.set_font("Icons".to_string()).class("zoom").class("last"));


        entity
    }
}

impl EventHandler for AppWidget {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) -> bool {
        

        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {

                // TODO
                // WindowEvent::GeometryChanged => {
                //     if event.target == entity {

                //         let total_samples = (state.transform.get_width(entity) * self.samples_per_pixel.round()) as i32;
                //         let mut new_end = total_samples;
                //         self.end = new_end.min(self.left_channel.len() as i32);
                //     }
                // }

                WindowEvent::MouseMove(x,_) => {
                    if event.target == entity {
                        //let samples_per_pixel = (self.end - self.start) as f32 / state.transform.get_width(entity);
                        //let samples_per_pixel = 220.0;

                        self.zoom_pos_pixel = *x - state.transform.get_posx(entity);
                        
                        self.zoom_pos = self.start + (self.samples_per_pixel.round() * self.zoom_pos_pixel) as i32;

                        if self.zoom_pos >= self.left_channel.len() as i32 {
                            self.zoom_pos = self.left_channel.len() as i32 - 1;
                        }

                        //println!("Zoom Sample: {}", self.zoom_pos);

                        state.insert_event(Event::new(WindowEvent::Redraw));

                        if self.left_channel.len() > 0 {
                            let time = self.zoom_pos as f32 / 44100.0;
                            let time_value : TimeValue = time.into();
                            let time_string = format!("Time: {}", time_value);
                            self.time_label.set_text(state, &time_string);

                            match self.units_mode {
                                UnitsMode::Linear => {
                                    let value = self.left_channel[self.zoom_pos as usize];
                                    let value_string = format!("Value: {:+.2e}", value);
                                    self.value_label.set_text(state, &value_string);
                                }

                                UnitsMode::Decibel => {
                                    let value = 10.0 * self.left_channel[self.zoom_pos as usize].abs().log10();
                                    let value_string = format!("Value: {:.2} dB", value);
                                    self.value_label.set_text(state, &value_string);
                                }
                            }
                            
                        }
                    
                        //println!("Zoom Pos: {}", self.zoom_pos);

                    }
                }

                WindowEvent::MouseScroll(_,y) => {
                    if *y > 0.0 {
                        if state.modifiers.ctrl {

                            if self.zoom_level != 14 {
                                self.zoom_level += 1;
                            }
                            
                            let zoom_ratio = ZOOM_LEVELS[self.zoom_level];

                            self.samples_per_pixel = 220.0 / zoom_ratio;

                            let total_samples = (state.transform.get_width(entity) * self.samples_per_pixel.round()) as i32;

                            let mut new_start = 0;
                            let mut new_end = total_samples;                        

                            let offset = self.zoom_pos - (self.zoom_pos_pixel * self.samples_per_pixel.round()) as i32;

                            new_start += offset;
                            new_end += offset;
                           

                            self.start = new_start.max(0).min(self.left_channel.len() as i32 - 1);
                            self.end = new_end.min(self.left_channel.len() as i32 - 1);


                        } else {
                         
                        
                            let mut new_start  = self.start + (self.samples_per_pixel * 30.0) as i32;
                            //let mut new_end =  self.end + (self.samples_per_pixel * 30.0) as i32;


                            // let offset = new_end - self.left_channel.len() as i32;
                            // if offset > 0 {
                            //     new_end = self.left_channel.len() as i32;
                            //     new_start = new_start - offset;
                            // }   
                            
                            self.start = new_start.max(0).min(self.end);

                            //self.end = new_end.min(self.left_channel.len() as i32);
                        }



                        state.insert_event(Event::new(WindowEvent::Redraw)); 
                        return true;
                    } else if *y < 0.0 {

                        if state.modifiers.ctrl {
                         
                            if self.zoom_level != 0 {
                                self.zoom_level -= 1;
                            }
                            
                            let zoom_ratio = ZOOM_LEVELS[self.zoom_level];

                            self.samples_per_pixel = 220.0 / zoom_ratio;

                            let total_samples = (state.transform.get_width(entity) * self.samples_per_pixel.round()) as i32;

                            let zoom_samples = (self.zoom_pos as f32 / (ZOOM_LEVELS[self.zoom_level + 1]/ZOOM_LEVELS[self.zoom_level])) as i32;


                            let offset = self.zoom_pos - (self.zoom_pos_pixel * self.samples_per_pixel.round()) as i32;


                            let new_start = 0 + offset;
                            let new_end = total_samples + offset;

                            self.start = new_start.max(0);
                            self.end = new_end.min(self.left_channel.len() as i32);


                            
                        } else {
                           
                            let mut new_start = self.start - (self.samples_per_pixel * 30.0) as i32;
                            //let mut new_end = self.end - (self.samples_per_pixel * 30.0) as i32;

                            // let offset = 0 - new_start;
                            // if offset > 0 {
                            //     new_end = new_end + offset;
                            //     new_start = 0;
                            // }     
                            
                            self.start = new_start.max(0).min(self.left_channel.len() as i32 - 1);
                            //self.end = new_end.min(self.left_channel.len() as i32);

                        }

                        state.insert_event(Event::new(WindowEvent::Redraw));
                        return true;
                    }
                }

                _=> {}
            }
        }

        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {

                AppEvent::LoadAudioFile(file_path) => {
                    self.read_audio(file_path);

                    let num_samples = self.left_channel.len();

                    let samples_per_pixel = 220.0;

                    println!("Calculated Samples Per Pixel: {}", samples_per_pixel);
                    self.end = (state.transform.get_width(entity) * samples_per_pixel).ceil() as i32;
                    if self.end > self.left_channel.len() as i32 {
                        self.end = self.left_channel.len() as i32;
                    }
                }

                AppEvent::OpenFileDialog => {

                    // Use this if nfd not working
                    // self.read_audio("sounds/guitar-tone.wav");

                    // let num_samples = self.left_channel.len();

                    // let samples_per_pixel = 220.0;

                    // self.end = (state.transform.get_width(entity) * samples_per_pixel).ceil() as i32;
                    // if self.end > self.left_channel.len() as i32 {
                    //     self.end = self.left_channel.len() as i32;
                    // }

                    // Comment this is nfd not working
                    {

                        let result = FileDialog::new().show_open_single_file().expect("Failed to open file dialog");
                    
                        match result {
                            Some(file_path) => {
                                println!("File path = {:?}", file_path);

                                self.read_audio(file_path.as_os_str().to_str().unwrap());

                                let num_samples = self.left_channel.len();

                                let samples_per_pixel = 220.0;

                                //println!("Calculated Samples Per Pixel: {}", samples_per_pixel);
                                self.end = (state.transform.get_width(entity) * samples_per_pixel).ceil() as i32;
                                if self.end > self.left_channel.len() as i32 {
                                    self.end = self.left_channel.len() as i32;
                                }
                            
                            },
                            // TODO
                            None => panic!("Invalid wav file path")
                        }  
                    }
                    

                    return true;
                }
                AppEvent::SwicthChannel(channel_mode) => {
                    self.channel_mode = channel_mode.clone();
                    state.insert_event(Event::new(WindowEvent::Redraw));

                }

                AppEvent::SwitchUnits(units_mode) => {
                    self.units_mode = units_mode.clone();
                    state.insert_event(Event::new(WindowEvent::Redraw));
                }
            }
        }
        
        false
    }

    fn on_draw(&mut self, state: &mut State, entity: Entity, canvas: &mut Canvas<OpenGl>) {
        


        let y = state.transform.get_posy(self.waveview);
        let h = state.transform.get_height(self.waveview);

        match self.channel_mode {
            ChannelMode::Left => {
                self.draw_channel(state, entity,&self.left_channel, y, h, canvas);
            }

            ChannelMode::Right => {
                self.draw_channel(state, entity,&self.right_channel, y, h, canvas);
            }

            ChannelMode::Both => {
                self.draw_channel(state, entity,&self.left_channel, y, h/2.0, canvas);
                self.draw_channel(state, entity,&self.right_channel, y + h/2.0, h/2.0, canvas);
            }
        }
    }
}


