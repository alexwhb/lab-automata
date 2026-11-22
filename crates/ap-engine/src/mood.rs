//! The Tray's "what is happening" classifier and its population history (SPEC TRAY-8, ENG-6).

/// The Tray keeps the populations of the last 120 generations.
pub const HISTORY_LEN: usize = 120;

/// The population of the last [`HISTORY_LEN`] generations, newest last — the Tray's `hist`.
///
/// It is never empty once made: it starts with the current population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    values: Vec<usize>,
}

impl History {
    /// A history holding just `population` (after Restart, Clear or Random soup).
    pub fn new(population: usize) -> History {
        History {
            values: vec![population],
        }
    }

    /// Appends a generation's population, dropping the oldest beyond 120 (after a step).
    pub fn push(&mut self, population: usize) {
        self.values.push(population);
        if self.values.len() > HISTORY_LEN {
            let extra = self.values.len() - HISTORY_LEN;
            self.values.drain(..extra);
        }
    }

    /// Replaces the newest value (after drawing on the current generation, as the mockup's
    /// `edit()` does).
    pub fn replace_last(&mut self, population: usize) {
        match self.values.last_mut() {
            Some(last) => *last = population,
            None => self.values.push(population),
        }
    }

    /// Starts over with one value.
    pub fn reset(&mut self, population: usize) {
        self.values.clear();
        self.values.push(population);
    }

    /// The values, oldest first (for the sparkline).
    pub fn values(&self) -> &[usize] {
        &self.values
    }

    /// The newest population.
    pub fn latest(&self) -> usize {
        self.values.last().copied().unwrap_or(0)
    }

    /// The largest value, at least 10 — the sparkline's vertical scale.
    pub fn scale_max(&self) -> usize {
        self.values.iter().copied().max().unwrap_or(0).max(10)
    }

    /// The mood of this history.
    pub fn mood(&self) -> Mood {
        Mood::classify(&self.values)
    }
}

impl Default for History {
    /// A history holding a single 0.
    fn default() -> Self {
        History::new(0)
    }
}

/// What the Tray thinks is happening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    /// Nothing is alive.
    Extinct,
    /// The population has been constant for 12+ generations.
    Settled,
    /// The population repeats with period 2 over the last 12+ generations.
    Oscillating,
    /// The population grew by more than 35 % over 20 generations.
    Booming,
    /// The population fell by more than 30 % over 20 generations.
    Collapsing,
    /// None of the above.
    Churning,
}

impl Mood {
    /// Classifies a population history (oldest first) exactly as the Tray mockup does: look at
    /// the newest 16 values and the growth against the value 20 generations back.
    pub fn classify(history: &[usize]) -> Mood {
        let Some(&pop) = history.last() else {
            // The mockup compares `undefined` and falls through to its default.
            return Mood::Churning;
        };
        let last = &history[history.len().saturating_sub(16)..];
        let growth = if history.len() > 20 {
            let then = history[history.len() - 21];
            (pop as f64 - then as f64) / then.max(1) as f64
        } else {
            0.0
        };
        let p2 = last.len() >= 12
            && last
                .iter()
                .enumerate()
                .all(|(i, &v)| i < 2 || v == last[i - 2]);
        if pop == 0 {
            Mood::Extinct
        } else if last.len() >= 12 && last.iter().all(|&v| v == last[0]) {
            Mood::Settled
        } else if p2 {
            Mood::Oscillating
        } else if growth > 0.35 {
            Mood::Booming
        } else if growth < -0.3 {
            Mood::Collapsing
        } else {
            Mood::Churning
        }
    }

    /// The callout's title ("Settled").
    pub const fn title(self) -> &'static str {
        match self {
            Mood::Extinct => "Extinct",
            Mood::Settled => "Settled",
            Mood::Oscillating => "Oscillating",
            Mood::Booming => "Booming",
            Mood::Collapsing => "Collapsing",
            Mood::Churning => "Churning",
        }
    }

    /// The callout's sentence.
    pub const fn text(self) -> &'static str {
        match self {
            Mood::Extinct => "Everything died. Stamp or draw to restart.",
            Mood::Settled => "Only still lifes left. Try hitting one with a glider.",
            Mood::Oscillating => "Repeating the same few states.",
            Mood::Booming => "Births are far outpacing deaths.",
            Mood::Collapsing => "Population is falling fast.",
            Mood::Churning => "Births and deaths are balanced.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_basic_shapes() {
        assert_eq!(Mood::classify(&[5, 0]), Mood::Extinct);
        assert_eq!(Mood::classify(&[7; 12]), Mood::Settled);
        assert_eq!(
            Mood::classify(&[7; 11]),
            Mood::Churning,
            "too short to call"
        );
        let p2: Vec<usize> = (0..14).map(|i| 10 + (i % 2) * 3).collect();
        assert_eq!(Mood::classify(&p2), Mood::Oscillating);
        let grow: Vec<usize> = (0..25).map(|i| 10 + i * 2).collect();
        assert_eq!(Mood::classify(&grow), Mood::Booming);
        let fall: Vec<usize> = (0..25).map(|i| 100 - i * 3).collect();
        assert_eq!(Mood::classify(&fall), Mood::Collapsing);
        assert_eq!(Mood::classify(&[]), Mood::Churning);
    }

    #[test]
    fn history_caps_at_120() {
        let mut h = History::new(3);
        for i in 0..200 {
            h.push(i);
        }
        assert_eq!(h.values().len(), HISTORY_LEN);
        assert_eq!(h.values()[0], 80);
        h.replace_last(0);
        assert_eq!(h.mood(), Mood::Extinct);
        assert_eq!(h.scale_max(), 198);
        h.reset(4);
        assert_eq!(h.values(), &[4]);
    }
}
