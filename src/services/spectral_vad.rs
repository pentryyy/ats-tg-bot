use crate::config::config::VadConfig;
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

const HAMMING_ALPHA: f32 = 0.54;
const HAMMING_BETA: f32 = 0.46;
const EPSILON: f32 = 0.001;

#[inline]
fn freq_to_bin(freq: u32, fft_size: usize, sample_rate: u32) -> usize {
    (freq * fft_size as u32 / sample_rate) as usize
}

pub struct SpectralVAD {
    fft: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex<f32>>,
    params: VadConfig,
}

impl SpectralVAD {
    pub fn new(params: VadConfig) -> Self {
        let fft_size = params.fft_size;
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); fft_size],
            params,
        }
    }

    pub fn detect_speech(&mut self, frame: &[i16], sample_rate: u32) -> bool {
        let n = self.buffer.len();
        assert!(frame.len() >= n, "Фрейм короче FFT размера");

        for i in 0..n {
            let window = HAMMING_ALPHA
                - HAMMING_BETA * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
            self.buffer[i] = Complex::new(frame[i] as f32 * window, 0.0);
        }

        self.fft.process(&mut self.buffer);

        let low_bin = freq_to_bin(self.params.low_freq, n, sample_rate);
        let speech_bin_start = freq_to_bin(self.params.speech_freq_start, n, sample_rate);
        let speech_bin_end = freq_to_bin(self.params.speech_freq_end, n, sample_rate);

        let mut low_energy = 0.0;
        let mut speech_energy = 0.0;

        for i in 0..n / 2 {
            let mag = self.buffer[i].norm();
            if i < low_bin {
                low_energy += mag;
            } else if i >= speech_bin_start && i <= speech_bin_end {
                speech_energy += mag;
            }
        }

        speech_energy > self.params.speech_energy_threshold
            && (speech_energy / (low_energy + EPSILON)) > self.params.ratio_threshold
    }
}
