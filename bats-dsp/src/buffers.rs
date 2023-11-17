use std::{fmt, path::Path};

use anyhow::{anyhow, Result};

use crate::sample_rate::SampleRate;

/// Buffers contains a left and right audio channel.
#[derive(Clone, PartialEq)]
pub struct Buffers {
    /// The left audio channel.
    pub left: Vec<f32>,
    /// The right audio channel.
    pub right: Vec<f32>,
}

impl Buffers {
    /// Create new zeroed buffers of size `len`.
    pub fn new(len: usize) -> Buffers {
        Buffers {
            left: vec![0.0; len],
            right: vec![0.0; len],
        }
    }

    /// Create a new buffer from an iterator.
    pub fn with_iter(iter: impl Iterator<Item = (f32, f32)>) -> Buffers {
        let mut left = Vec::with_capacity(iter.size_hint().1.unwrap_or(0));
        let mut right = Vec::with_capacity(iter.size_hint().1.unwrap_or(0));
        for (l, r) in iter {
            left.push(l);
            right.push(r);
        }
        Buffers { left, right }
    }

    /// Create new buffers from a wav file. `sample_rate` should be the sample rate of the returned `Buffers`.
    ///
    /// # TODO
    /// Support mono, other formats, and sample rate conversion.
    pub fn from_wav(p: impl AsRef<Path>, sample_rate: SampleRate) -> Result<Buffers> {
        let reader = hound::WavReader::open(p.as_ref())
            .map_err(|err| anyhow!("Could not read from {:?} with error: {}", p.as_ref(), err))?;
        if reader.spec().sample_rate != sample_rate.sample_rate() as u32 {
            return Err(anyhow!(
                "expected sample rate {} but got {} from {:?}",
                sample_rate.sample_rate(),
                reader.spec().sample_rate,
                p.as_ref(),
            ));
        }
        if reader.spec().channels != 2 {
            return Err(anyhow!(
                "only 2 channels are supported but got {} from {:?}",
                reader.spec().channels,
                p.as_ref()
            ));
        }
        let mut buffers = Buffers {
            left: Vec::with_capacity(reader.duration() as usize),
            right: Vec::with_capacity(reader.duration() as usize),
        };
        let mut samples = reader.into_samples::<i32>();
        while let Some(s) = samples.next() {
            let convert_sample = |v| v as f32 / i32::MAX as f32;
            buffers.left.push(convert_sample(s?));
            buffers.right.push(convert_sample(samples.next().unwrap()?));
        }
        Ok(buffers)
    }

    /// Get the samples at `idx`.
    pub fn get(&self, idx: usize) -> (f32, f32) {
        (
            self.left.get(idx).copied().unwrap_or_default(),
            self.right.get(idx).copied().unwrap_or_default(),
        )
    }

    /// Set the samples at `idx`.
    pub fn set(&mut self, idx: usize, samples: (f32, f32)) {
        self.left[idx] = samples.0;
        self.right[idx] = samples.1;
    }

    /// The length of the buffers.
    pub fn len(&self) -> usize {
        self.left.len().min(self.right.len())
    }

    /// Returns true if this is an empty buffer.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if all samples are `0.0`.
    pub fn is_zero(&self) -> bool {
        self.left.iter().all(|v| *v == 0.0) && self.right.iter().all(|v| *v == 0.0)
    }
}

impl fmt::Debug for Buffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Don't display the whole array on debug as it is usually too long to be useful.
        let display_len = self.len().min(4);
        f.debug_struct("Buffers")
            .field("length", &self.left.len())
            .field("left", &&self.left[0..display_len])
            .field("right", &&self.right[0..display_len])
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn new_buffers_is_zerod() {
        let b = Buffers::new(1024);
        assert!(!b.is_empty());
        assert_eq!(b.len(), 1024);
        assert_eq!(b.left, vec![0.0; 1024]);
        assert_eq!(b.right, vec![0.0; 1024]);
    }

    #[test]
    fn read_supported_wav_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../assets/test/stereo_44100_32bit_signed.wav");
        let data = Buffers::from_wav(path, SampleRate::new(44100.0)).unwrap();
        // 1 second at 44.1kHz should hav 44100 samples.
        assert_eq!(data.left.len(), 44100);
        assert_eq!(data.right.len(), 44100);
    }

    #[test]
    fn read_mono_wav_file_returns_error() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../assets/test/mono_44100_32bit_signed.wav");
        assert!(Buffers::from_wav(path, SampleRate::new(44100.0)).is_err());
    }

    #[test]
    fn read_wav_file_on_unsupported_sample_rate_produces_error() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../assets/test/stereo_44100_32bit_signed.wav");
        assert!(Buffers::from_wav(path, SampleRate::new(88200.0)).is_err());
    }

    #[test]
    fn read_wav_from_file_that_does_not_exist_produces_error() {
        assert!(Buffers::from_wav("/does/not/exist", SampleRate::new(44100.0)).is_err());
    }

    #[test]
    fn get_out_of_range_returns_zeros() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../assets/test/stereo_44100_32bit_signed.wav");
        let data = Buffers::from_wav(path, SampleRate::new(44100.0)).unwrap();
        assert_eq!(data.get(usize::MAX), (0.0, 0.0));
    }

    #[test]
    fn set_sample_sets_the_sample() {
        let mut buffers = Buffers::with_iter(std::iter::repeat((1.0, 1.0)).take(100));
        assert_eq!(buffers.get(10), (1.0, 1.0));
        buffers.set(10, (-1.0, -1.0));
        assert_eq!(buffers.get(10), (-1.0, -1.0));
    }

    #[test]
    fn debug_buffers() {
        assert!(format!("{:?}", Buffers::new(1024)).len() < 1024);
        assert!(format!("{:?}", Buffers::new(1)).len() > 0);
    }
}
