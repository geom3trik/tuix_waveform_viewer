
extern crate tuix;

// use std::{fs::read, path::Path};

const ICON_TO_START: &str = "\u{23ee}";
const ICON_PLAY: &str = "\u{25b6}";
const ICON_STOP: &str = "\u{25a0}";
const ICON_TO_END: &str = "\u{23ed}";

use tuix::*;

extern crate nfd2;

use nfd2::Response;

use std::{cmp::Ordering, println};

use dasp_sample::{Sample, I24};

mod audio;
pub use audio::*;

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

        AppWidget::new().build(state, window, |builder| builder.class("app"));

        win_desc.with_title("Waveform Viewer").with_inner_size(1000, 600)
    });

    app.run();
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckListEvent {
    Switch,
}

pub struct CheckList {

}

impl CheckList {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl BuildHandler for CheckList {
    type Ret = Entity;
    fn on_build(&mut self, _state: &mut State, entity: Entity) -> Self::Ret {

        entity
    }
}

impl EventHandler for CheckList {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) -> bool {
        if let Some(checklist_event) = event.message.downcast::<CheckListEvent>() {
            match checklist_event {
                CheckListEvent::Switch => {
                    if event.target == entity {
                        for child in entity.child_iter(&state.hierarchy.clone()) {
                            child.set_checked(state, false);
                        }

                        event.origin.set_checked(state, true);                        
                    }


                }
            }
        }

        false
    }
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
pub enum AppEvent {
    OpenFileDialog,
    SwicthMode(ChannelMode),
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
    is_decibel: bool,

    time_label: Entity,
    value_label: Entity,
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
            is_decibel: true,

            time_label: Entity::null(),
            value_label: Entity::null(),
            
        }
    }
}

impl AppWidget {
    // Opens the file specified by self.wave_file_path, reads the audio samples into self.audio
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

    fn draw_channel(&self, state: &mut State, entity: Entity, data: &[f32], posy: f32, height: f32, canvas: &mut Canvas<OpenGl>) {
        let x = state.transform.get_posx(entity);
        let y = posy;
        let w = state.transform.get_width(entity);
        let h = height;

        //let samples_per_pixel = (self.end - self.start) as f32 / w;
        
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
                    
                    //if pixel  % pixels_per_sample == 0 {

                        let sample = self.start + (self.samples_per_pixel * pixel as f32).floor() as i32;
                        path.move_to(x + (pixel as f32), y + h/2.0);
                        path.line_to(x + (pixel as f32), y + h/2.0 - data[sample as usize]* h/2.0);
                    //}
                }
            } else {
    
                let mut chunks = audio.chunks(self.samples_per_pixel.round() as usize);
    
                for chunk in 0..w as u32 {
        
                    if let Some(c) = chunks.next() {
                        let v_min = *c.iter().min_by(|a,b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap();
                        let v_max = *c.iter().max_by(|a,b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap();

                        if self.is_decibel {

                            let mut v_min_db = (1.0 + (20.0 * v_min.abs().log10()).max(-60.0) / 60.0);
                            let mut v_max_db = (1.0 + (20.0 * v_max.abs().log10()).max(-60.0) / 60.0);

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

                            //let v_min_db = (20.0 * v_min.abs().log10()).max(-60.0) / -60.0;
                            //let v_max_db = (20.0 * v_max.abs().log10()).max(-60.0) / -60.0;
                            //println!("{} {} {} {}", v_min, v_max, v_min_db, v_max_db);
                            path.line_to(x + (chunk as f32), y + h/2.0 - v_min_db * h/2.0);
                            path.line_to(x + (chunk as f32), y + h/2.0 - v_max_db * h/2.0);
                        } else {
                            path.line_to(x + (chunk as f32), y + h/2.0 - v_min * h/2.0);
                            path.line_to(x + (chunk as f32), y + h/2.0 - v_max * h/2.0);
                        }
                        
                        
                    }
                }
            }
    
    
            // path.line_to(x + w, y + h);
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

        entity.set_flex_grow(state, 1.0).set_flex_direction(state, FlexDirection::Row);

        Button::new().on_release(Event::new(AppEvent::OpenFileDialog)).build(state, entity, |builder| {
            builder.set_text("Open").set_margin(Length::Pixels(10.0))
        });

        let checklist = CheckList::new().build(state, entity, |builder| builder.class("checklist"));

        Button::new().build(state, checklist, |builder| {
            builder.set_text(ICON_TO_START).class("first")
        });

        let play =  Button::new().build(state, checklist, |builder| {
            builder.set_text(ICON_PLAY).class("play")
        });

        Button::new().build(state, checklist, |builder| {
            builder.set_text(ICON_STOP)
        });

        Button::new().build(state, checklist, |builder| {
            builder.set_text(ICON_TO_END).class("last")
        });

        let checklist = CheckList::new().build(state, entity, |builder| builder.class("checklist"));

        let left = Button::new().on_press(Event::new(AppEvent::SwicthMode(ChannelMode::Left)).target(entity)).build(state, checklist, |builder| {
            builder.set_text("L").class("first")
        });

        left.set_checked(state, true);

        Button::new().on_press(Event::new(AppEvent::SwicthMode(ChannelMode::Right)).target(entity)).build(state, checklist, |builder| {
            builder.set_text("R")
        });

        Button::new().on_press(Event::new(AppEvent::SwicthMode(ChannelMode::Both)).target(entity)).build(state, checklist, |builder| {
            builder.set_text("L + R").class("last").set_width(Length::Pixels(60.0))
        });

        self.time_label = Label::new("Time: -").build(state, entity, |builder| builder.set_margin(Length::Pixels(10.0)));
        self.value_label = Label::new("Value: -").build(state, entity, |builder| builder.set_margin(Length::Pixels(10.0)));

        let checklist = CheckList::new().build(state, entity, |builder| builder.class("checklist"));

        let linear = Button::new().on_press(Event::new(CheckListEvent::Switch).target(checklist)).build(state, checklist, |builder| {
            builder.set_text("Mag").class("first")
        });

        linear.set_checked(state, true);

        Button::new().on_press(Event::new(CheckListEvent::Switch).target(checklist)).build(state, checklist, |builder| {
            builder.set_text("dB").class("last")
        });


        entity
    }
}

impl EventHandler for AppWidget {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) -> bool {
        

        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {

                WindowEvent::MouseMove(x,_) => {
                    if event.target == entity {
                        //let samples_per_pixel = (self.end - self.start) as f32 / state.transform.get_width(entity);
                        //let samples_per_pixel = 220.0;

                        self.zoom_pos_pixel = *x - state.transform.get_posx(entity);
                        
                        self.zoom_pos = self.start + (self.samples_per_pixel.round() * self.zoom_pos_pixel) as i32;

                        if self.zoom_pos > self.left_channel.len() as i32 {
                            self.zoom_pos = self.left_channel.len() as i32 - 1;
                        }

                        //println!("Zoom Sample: {}", self.zoom_pos);

                        state.insert_event(Event::new(WindowEvent::Redraw));

                        if self.left_channel.len() > 0 {
                            let time = self.zoom_pos as f32 / 44100.0;
                            let time_value : TimeValue = time.into();
                            let time_string = format!("Time: {}", time_value);
                            self.time_label.set_text(state, &time_string);

                            let value = self.left_channel[self.zoom_pos as usize];
                            let value_string = format!("Value: {:+.2e}", value);
                            self.value_label.set_text(state, &value_string);
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

                            let zoom_samples = (self.zoom_pos as f32 / (ZOOM_LEVELS[self.zoom_level]/ZOOM_LEVELS[self.zoom_level - 1])) as i32;

                            let mut new_start = 0;
                            let mut new_end = total_samples;

                            println!("SPP: {} ZR: {} TS: {} ZP: {} ZS: {} NS: {} NE: {}", self.samples_per_pixel.round(), zoom_ratio, total_samples, self.zoom_pos, zoom_samples, new_start, new_end);
                            
                            //println!("Old End: {}", self.end);
                            //let mut new_start = ((self.start + self.zoom_pos) as f32 / zoom_ratio) as i32;
                            //let mut new_end = ((self.end + self.zoom_pos) as f32 / zoom_ratio) as i32;
                            //let mut new_end = (state.transform.get_width(entity) * self.samples_per_pixel).ceil() as i32;
                            //println!("Samples Per Pixel: {}  End: {}", self.samples_per_pixel, new_end);
                            //let mut new_start = self.start + (self.zoom_pos - self.start) / 2;
                            //let mut new_end = self.end - (self.end - self.zoom_pos) / 2;

                            //let samples_per_pixel = (new_end - new_start) as f32 / state.transform.get_width(entity);

                            let offset = self.zoom_pos - (self.zoom_pos_pixel * self.samples_per_pixel.round()) as i32;


                         
                            //println!("Samples Per Pixel: {}  Zoom Pos: {}  Zoom Pixel: {}  Offset: {}", self.samples_per_pixel, self.zoom_pos, self.zoom_pos_pixel, offset);
                            new_start += offset;
                            new_end += offset;
                            //println!("Offset: {}", ZOOM_LEVELS[self.zoom_level]);

                            self.start = new_start.max(0);
                            self.end = new_end.min(self.left_channel.len() as i32);

                            // self.zoom_pos_pixel = state.mouse.cursorx - state.transform.get_posx(entity);
                        
                            // self.zoom_pos = self.start + (self.samples_per_pixel * self.zoom_pos_pixel).ceil() as i32;

                            // if self.zoom_pos > self.left_channel.len() as i32 {
                            //     self.zoom_pos = self.left_channel.len() as i32 - 1;
                            // }


                        } else {
                         
                            //let samples_per_pixel = (self.end - self.start) as f32 / state.transform.get_width(entity);
                            let mut new_start  = self.start + (self.samples_per_pixel * 30.0) as i32;
                            let mut new_end =  self.end + (self.samples_per_pixel * 30.0) as i32;


                            let offset = new_end - self.left_channel.len() as i32;
                            if offset > 0 {
                                new_end = self.left_channel.len() as i32;
                                new_start = new_start - offset;
                            }   
                            
                            self.start = new_start.max(0);
                            self.end = new_end.min(self.left_channel.len() as i32);

                            println!("Start: {}  End: {} SPP: {}", self.start, self.end, self.samples_per_pixel);
                        }



                        state.insert_event(Event::new(WindowEvent::Redraw)); 
                        return true;
                    } else if *y < 0.0 {

                        if state.modifiers.ctrl {
                         
                            // let mut new_start = self.start - (self.zoom_pos - self.start) * 2;
                            // let mut new_end = self.end + (self.end - self.zoom_pos) * 2;
                        

                            // let samples_per_pixel = (new_end - new_start) as f32 / state.transform.get_width(entity);
                            // let offset = self.zoom_pos - (self.zoom_pos_pixel * samples_per_pixel) as i32 - new_start;
                    
                            // new_start += offset;
                            // new_end += offset;
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

                            println!("SPP: {} ZR: {} TS: {} ZP: {} ZS: {} NS: {} NE: {}", self.samples_per_pixel.round(), zoom_ratio, total_samples, self.zoom_pos, zoom_samples, new_start, new_end);
                            
                            

                            self.start = new_start.max(0);
                            self.end = new_end.min(self.left_channel.len() as i32);


                            
                        } else {
                           
                            //let samples_per_pixel = (self.end - self.start) as f32 / state.transform.get_width(entity);
                            let mut new_start = self.start - (self.samples_per_pixel * 30.0) as i32;
                            let mut new_end = self.end - (self.samples_per_pixel * 30.0) as i32;

                            let offset = 0 - new_start;
                            if offset > 0 {
                                new_end = new_end + offset;
                                new_start = 0;
                            }     
                            
                            self.start = new_start.max(0);
                            self.end = new_end.min(self.left_channel.len() as i32);

                            println!("Start: {}  End: {}", self.start, self.end);
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
                AppEvent::OpenFileDialog => {
                    let result = nfd2::open_file_dialog(None, None).unwrap_or_else(|e| {
                        panic!(e);
                    });
                  
                    match result {
                        Response::Okay(file_path) => {
                            println!("File path = {:?}", file_path);

                            self.read_audio(file_path.as_os_str().to_str().unwrap());

                            //self.zoom_level = 64;

                            let num_samples = self.left_channel.len();

                            let samples_per_pixel = 220.0;

                            //let mut samples_per_pixel = num_samples as f32 / state.transform.get_width(entity);

                            //let zoom_level = (4410.0 / samples_per_pixel).ceil();
                            //self.zoom_level = 2usize.pow(zoom_level.log2().ceil() as u32) as usize;

                            //samples_per_pixel = (4410.0 / self.zoom_level as f32).ceil();

                            println!("Calculated Samples Per Pixel: {}", samples_per_pixel);
                            self.end = (state.transform.get_width(entity) * samples_per_pixel).ceil() as i32;
                            if self.end > self.left_channel.len() as i32 {
                                self.end = self.left_channel.len() as i32;
                            }
                        
                        },
                        Response::OkayMultiple(files) => println!("Files {:?}", files),
                        Response::Cancel => println!("User canceled"),
                    }

                    return true;
                }
                AppEvent::SwicthMode(ch_mode) => {
                    self.channel_mode = ch_mode.clone();
                    state.insert_event(Event::new(WindowEvent::Redraw));

                }
            }
        }
        
        false
    }

    fn on_draw(&mut self, state: &mut State, entity: Entity, canvas: &mut Canvas<OpenGl>) {
        
        let y = state.transform.get_posy(entity);
        let h = state.transform.get_height(entity);

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


