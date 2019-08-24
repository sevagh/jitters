use crate::rtp::JITTERS_SAMPLE_RATE;

pub fn samples_to_ms(samples: usize, channels: u16) -> f64 {
    (1000.0 / (f64::from(JITTERS_SAMPLE_RATE))) * (samples as f64 / (f64::from(2 * channels)))
}
