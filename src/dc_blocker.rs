pub(crate) struct DcBlocker {
    enabled: bool,
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self {
            enabled: true,
            coefficient: (-std::f32::consts::TAU * 20.0 / sample_rate as f32).exp(),
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    #[cfg(any(feature = "desktop", test))]
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.previous_input = 0.0;
            self.previous_output = 0.0;
        }
    }

    pub(crate) fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }
        // First-order high-pass: y[n] = x[n] - x[n-1] + R * y[n-1].
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_blocker_is_enabled_by_default_and_removes_constant_signals() {
        for rate in [44_100, 48_000, 61_440, 96_000] {
            for dc in [-1.0, -0.25, 0.25, 1.0] {
                let mut filter = DcBlocker::new(rate);
                assert!(filter.enabled);
                assert_eq!(filter.process(dc), dc);
                let mut output = dc;
                for _ in 0..rate / 4 {
                    output = filter.process(dc);
                }
                assert!(output.abs() < 0.00001, "{rate} Hz, DC={dc}: {output}");
            }
        }
    }

    #[test]
    fn bypass_is_exact_and_reenabling_resets_filter_history() {
        let mut filter = DcBlocker::new(48_000);
        for _ in 0..1024 {
            filter.process(0.75);
        }
        filter.set_enabled(false);
        for sample in [-1.0, 0.0, 0.25, 1.0, -0.5] {
            assert_eq!(filter.process(sample), sample);
        }
        filter.set_enabled(true);
        let mut fresh = DcBlocker::new(48_000);
        for sample in [0.25, 0.25, -0.5, 1.0, 0.0] {
            assert_eq!(filter.process(sample), fresh.process(sample));
        }
    }

    #[test]
    fn dc_blocker_preserves_audio_and_attenuates_subsonic_frequencies() {
        let rate = 48_000;
        for (frequency, minimum, maximum) in [(1.0, 0.04, 0.06), (1000.0, 0.99, 1.01)] {
            let mut filter = DcBlocker::new(rate);
            let mut input_energy = 0.0;
            let mut output_energy = 0.0;
            for index in 0..rate * 2 {
                let input =
                    (std::f32::consts::TAU * frequency * index as f32 / rate as f32).sin() * 0.25;
                let output = filter.process(input);
                if index >= rate {
                    input_energy += input * input;
                    output_energy += output * output;
                }
            }
            let gain = (output_energy / input_energy).sqrt();
            assert!(
                (minimum..=maximum).contains(&gain),
                "{frequency} Hz gain: {gain}"
            );
        }
    }
}
