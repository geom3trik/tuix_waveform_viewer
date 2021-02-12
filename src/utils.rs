use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// A boolean flag that can be shared between threads
#[derive(Clone)]
#[allow(dead_code)]
pub struct Flag {
    flag: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl Flag {
    /// create a new flag, set to `false`
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// set the flag
    pub fn set(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// reset the flag
    #[allow(dead_code)]
    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    /// check if the flag is set
    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Interleave a buffer of samples into an output buffer.
pub fn interleave<T: Copy>(input: &[T], output: &mut [T], num_channels: usize) {
    debug_assert_eq!(input.len(), output.len());
    let num_samples = input.len() / num_channels;
    for sm in 0..num_samples {
        for ch in 0..num_channels {
            output[sm * num_channels + ch] = input[ch * num_samples + sm];
        }
    }
}

/// Deinterleave a buffer of samples into an output buffer
pub fn deinterleave<T: Copy>(input: &[T], output: &mut [T], num_channels: usize) {
    debug_assert_eq!(input.len(), output.len());
    let num_samples = input.len() / num_channels;
    for sm in 0..num_samples {
        for ch in 0..num_channels {
            output[ch * num_samples + sm] = input[sm * num_channels + ch];
        }
    }
}
