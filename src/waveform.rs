// Eventually this will be a widget dedicated to displaying waveforms

use std::cmp::Ordering;

pub fn to_u8(val: f32) -> u16 {
    (((val + 1.0) / 2.0) * std::u16::MAX as f32) as u16
}

pub fn to_f32(val: u16) -> f32 {
    ((val as f32 / std::u16::MAX as f32) * 2.0) - 1.0
}


pub const SAMPLES_PER_PIXEL: [usize; 9] = [
    4410, 1764, 882, 441, 147, 49, 21, 9, 3
];

pub struct Waveform {
    pub index: Vec<usize>,
    pub data: Vec<(u16, u16, u16)>,
}

impl Waveform {
    pub fn new() -> Self {
        Self {
            index: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn load(&mut self, audio: &[f32], num_of_pixels: usize)  {
        self.data.clear();
        self.index.clear();
        for level in 0..SAMPLES_PER_PIXEL.len() + 1 {
            self.index.push(self.data.len());
            let samples_per_pixel = if level == SAMPLES_PER_PIXEL.len() {
                audio.len() / num_of_pixels
            } else {
                SAMPLES_PER_PIXEL[level]
            };

            let chunks = audio.chunks(samples_per_pixel);
            for chunk in chunks {
                let v_min = *chunk
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap();
                let v_max = *chunk
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap();
                let v_mean: f32 = (chunk.iter().map(|s| s*s).sum::<f32>() / chunk.len() as f32).sqrt();
                self.data.push((to_u8(v_min), to_u8(v_max), to_u8(v_mean)));
            }
        }
    }

    pub fn set_num_pixels(&mut self, audio: &[f32], num_of_pixels: usize) {
        if num_of_pixels > 0 {
            if let Some(last) = self.index.last() {
                let samples_per_pixel = audio.len() / num_of_pixels;
                let chunks = audio.chunks(samples_per_pixel);
                for (idx, chunk) in chunks.enumerate() {
                    let v_min = *chunk
                        .iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                        .unwrap();
                    let v_max = *chunk
                        .iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                        .unwrap();
                    let v_mean: f32 = (chunk.iter().map(|s| s*s).sum::<f32>() / chunk.len() as f32).sqrt();
                    if last + idx < self.data.len() {
                        self.data[last + idx] = (to_u8(v_min), to_u8(v_max), to_u8(v_mean))
                    } else {
                        self.data.push((to_u8(v_min), to_u8(v_max), to_u8(v_mean)));
                    }
                    
                }                
            }         
        }

    }

    pub fn get_data(&self, level: usize) -> &[(u16, u16, u16)] {
        let index = self.index[level];
        let next_index = if level < SAMPLES_PER_PIXEL.len() {
            self.index[level+1]
        } else {
            self.data.len()
        };

        //println!("level: {} index: {} next: {} {}", level, index, next_index, self.data[index..next_index-1].len());

        return &self.data[index..next_index-1];

    }
} 