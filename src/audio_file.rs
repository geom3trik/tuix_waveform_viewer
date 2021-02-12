use crate::utils::deinterleave;
use hound::{SampleFormat, WavReader};

/// An audio file, loaded into memory
pub struct AudioFile {
    /// The sample data
    pub data: Vec<f32>,
    /// Sample rate of the audio file
    pub sample_rate: f64,
    /// number of channels in the audio file
    pub num_channels: usize,
    /// number of sample sin the audio file
    pub num_samples: usize,
}

impl AudioFile {
    /// return a buffer of samples corresponding to a channel in the audio file
    #[allow(dead_code)]
    pub fn get_channel(&self, idx: usize) -> &'_ [f32] {
        debug_assert!(idx < self.num_channels);
        let start = self.num_samples * idx;
        &self.data[start..(start + self.num_samples)]
    }

    /// open a file
    pub fn open(path: &str) -> Result<Self, hound::Error> {
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();
        let mut data = Vec::with_capacity((spec.channels as usize) * (reader.duration() as usize));
        match (spec.bits_per_sample, spec.sample_format) {
            (16, SampleFormat::Int) => {
                for sample in reader.samples::<i16>() {
                    data.push((sample? as f32) / (0x7fffi32 as f32));
                }
            }
            (24, SampleFormat::Int) => {
                for sample in reader.samples::<i32>() {
                    let val = (sample? as f32) / (0x00ff_ffffi32 as f32);
                    data.push(val);
                }
            }
            (32, SampleFormat::Int) => {
                for sample in reader.samples::<i32>() {
                    data.push((sample? as f32) / (0x7fff_ffffi32 as f32));
                }
            }
            (32, SampleFormat::Float) => {
                for sample in reader.samples::<f32>() {
                    data.push(sample?);
                }
            }
            _ => return Err(hound::Error::Unsupported),
        }

        let mut deinterleaved = vec![0.0; data.len()];
        let num_channels = spec.channels as usize;
        let num_samples = deinterleaved.len() / num_channels;
        deinterleave(&data, &mut deinterleaved, num_channels);
        Ok(Self {
            data: deinterleaved,
            sample_rate: spec.sample_rate as f64,
            num_channels,
            num_samples,
        })
    }
}
