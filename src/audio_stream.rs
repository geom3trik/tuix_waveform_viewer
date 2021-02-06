use crate::utils::interleave;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Stream;

/// The playback context is used by the audio callback to map data from the audio
/// file to the playback buffer.
pub struct PlaybackContext<'a> {
    pub buffer_size: usize,
    pub sample_rate: f64,
    pub num_channels: usize,
    output_buffer: &'a mut [f32],
}

impl<'a> PlaybackContext<'a> {
    /// return a buffer of output samples corresponding to a channel index
    pub fn get_output(&mut self, idx: usize) -> &'_ mut [f32] {
        let offset = idx * self.buffer_size;
        &mut self.output_buffer[offset..offset + self.buffer_size]
    }
}

/// start the audio stream
pub fn audio_stream(mut main_callback: impl FnMut(PlaybackContext) + Send + 'static) -> Stream {
    
    
    println!("Audio Stream");
    
    let host = cpal::default_host();
    let output_device = host.default_output_device().expect("no output found");
    let config = output_device
        .default_output_config()
        .expect("no default output config")
        .config();

    let sample_rate = config.sample_rate.0 as f64;
    let num_channels = config.channels as usize;
    let mut output_buffer = vec![];
    let mut input_buffer = vec![];

    output_buffer.resize_with(1 << 16, || 0.0);
    input_buffer.resize_with(1 << 16, || 0.0);

    let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let buffer_size = data.len() / num_channels;
        output_buffer.resize(data.len(), 0.0);
        for sample in data.iter_mut() {
            *sample = 0.0;
        }
        for sample in &mut output_buffer.iter_mut() {
            *sample = 0.0;
        }

        let context = PlaybackContext {
            buffer_size,
            num_channels,
            sample_rate,
            output_buffer: &mut output_buffer,
        };

        main_callback(context);
        interleave(&output_buffer, data, num_channels);
    };

    output_device
        .build_output_stream(&config, callback, |err| eprintln!("{}", err))
        .expect("failed to open stream")
}