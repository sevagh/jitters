use crate::rtp::JITTERS_SAMPLE_RATE;

pub fn samples_to_ms(samples: usize, channels: u16) -> f64 {
    return ((1000.0 / (JITTERS_SAMPLE_RATE as f64)) * (samples as f64 / ((2 * channels) as f64)))
        as f64;
}
