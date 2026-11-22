//! CSS timing functions and the small time-based animations of the bench: the tray's tilt
//! transition (`transition: transform .4s ease`), the loom spools (`spin 2.4s linear infinite`)
//! and the terrarium jars' bob (`bob 3s ease-in-out infinite`, delayed 0.4 s per jar).

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing function evaluated at progress `t ∈ [0, 1]`.
///
/// Solves `x(s) = t` for the curve parameter with Newton steps (falling back to bisection),
/// then returns `y(s)`, like browsers do.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    // polynomial coefficients of B(s) = 3(1−s)²s·p1 + 3(1−s)s²·p2 + s³
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let x = |s: f32| ((ax * s + bx) * s + cx) * s;
    let dx = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    let y = |s: f32| ((ay * s + by) * s + cy) * s;
    let mut s = t;
    for _ in 0..8 {
        let err = x(s) - t;
        if err.abs() < 1e-6 {
            return y(s);
        }
        let d = dx(s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= err / d;
    }
    // bisection
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    s = t;
    for _ in 0..40 {
        let v = x(s);
        if (v - t).abs() < 1e-6 {
            break;
        }
        if v < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    y(s)
}

/// CSS `ease` = `cubic-bezier(.25, .1, .25, 1)` — the tray's tilt transition.
pub fn ease(t: f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0, t)
}

/// CSS `ease-in-out` = `cubic-bezier(.42, 0, .58, 1)` — the jars' bob.
pub fn ease_in_out(t: f32) -> f32 {
    cubic_bezier(0.42, 0.0, 0.58, 1.0, t)
}

/// CSS `ease-out` = `cubic-bezier(0, 0, .58, 1)` — the cells' lift transition.
pub fn ease_out(t: f32) -> f32 {
    cubic_bezier(0.0, 0.0, 0.58, 1.0, t)
}

/// A CSS-style transition of one number, driven by frame time.
///
/// ```
/// use ap_bench::anim::{Transition, ease};
/// let mut tilt = Transition::new(1.0, 0.4, ease); // tilted
/// tilt.set_target(0.0);                             // "Top-down view" on
/// while tilt.step(1.0 / 60.0) {}                    // the app steps it each frame
/// assert_eq!(tilt.value(), 0.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Transition {
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
    /// The duration of a full-range change; [`Transition::set_target`] shortens it for partial
    /// changes (Chrome's "reversing shortening factor").
    full_duration: f32,
    timing: fn(f32) -> f32,
}

impl Transition {
    /// A transition resting at `value`; changes take `duration` seconds with `timing`.
    pub fn new(value: f32, duration: f32, timing: fn(f32) -> f32) -> Transition {
        Transition {
            from: value,
            to: value,
            elapsed: duration,
            duration,
            full_duration: duration,
            timing,
        }
    }

    /// The current (eased) value.
    pub fn value(&self) -> f32 {
        if self.duration <= 0.0 || self.elapsed >= self.duration {
            return self.to;
        }
        let p = (self.timing)(self.elapsed / self.duration);
        self.from + (self.to - self.from) * p
    }

    /// The value the transition is heading to.
    pub fn target(&self) -> f32 {
        self.to
    }

    /// Starts moving towards `to` from the current value. A change that covers only part of
    /// the 0‥1 range (a reversal half-way) takes proportionally less time.
    pub fn set_target(&mut self, to: f32) {
        if (to - self.to).abs() < f32::EPSILON {
            return;
        }
        let now = self.value();
        let span = (to - now).abs().min(1.0);
        self.from = now;
        self.to = to;
        self.elapsed = 0.0;
        self.duration = self.full_duration * span;
    }

    /// Jumps to `value` without animating.
    pub fn snap(&mut self, value: f32) {
        self.from = value;
        self.to = value;
        self.elapsed = self.duration;
    }

    /// Advances by `dt` seconds; returns `true` while still animating (request a repaint).
    pub fn step(&mut self, dt: f32) -> bool {
        if self.is_animating() {
            self.elapsed += dt.max(0.0);
        }
        self.is_animating()
    }

    /// `true` until the value has reached its target.
    pub fn is_animating(&self) -> bool {
        self.duration > 0.0 && self.elapsed < self.duration
    }
}

/// The loom spools' period: `.spin { animation: spin 2.4s linear infinite }`.
pub const SPOOL_PERIOD_S: f32 = 2.4;

/// The spool angle in radians after `seconds` of weaving (advance only while weaving: the
/// mockup pauses the animation with the thread).
pub fn spool_angle(seconds: f64) -> f32 {
    let turns = (seconds / SPOOL_PERIOD_S as f64).fract();
    (turns * std::f64::consts::TAU) as f32
}

/// The jars' bob: `@keyframes bob { 0%,100% { translateY(0) } 50% { translateY(-3px) } }`,
/// `3s ease-in-out infinite`, `animation-delay: index · 0.4s`. Returns the vertical offset in
/// CSS px (0 … −3) of jar `index` at `seconds` since the screen appeared.
pub fn jar_bob(seconds: f64, index: usize) -> f32 {
    let t = seconds - index as f64 * 0.4;
    if t < 0.0 {
        return 0.0;
    }
    let phase = (t / 3.0).fract() as f32;
    // the timing function applies per keyframe interval
    if phase < 0.5 {
        -3.0 * ease_in_out(phase / 0.5)
    } else {
        -3.0 * (1.0 - ease_in_out((phase - 0.5) / 0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_matches_known_values() {
        // reference values of CSS `ease` (Chrome's UnitBezier)
        assert!((ease(0.25) - 0.4094).abs() < 2e-3, "{}", ease(0.25));
        assert!((ease(0.5) - 0.8024).abs() < 2e-3, "{}", ease(0.5));
        assert!((ease(0.75) - 0.9604).abs() < 2e-3, "{}", ease(0.75));
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-4);
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
    }

    #[test]
    fn transition_reaches_target_in_duration() {
        let mut t = Transition::new(1.0, 0.4, ease);
        assert!(!t.is_animating());
        t.set_target(0.0);
        let mut frames = 0;
        while t.step(0.1) {
            frames += 1;
            assert!(t.value() > 0.0 && t.value() < 1.0);
        }
        assert_eq!(frames, 3);
        assert_eq!(t.value(), 0.0);
    }

    #[test]
    fn reversal_half_way_is_shorter() {
        let mut t = Transition::new(1.0, 0.4, |x| x);
        t.set_target(0.0);
        t.step(0.2);
        assert!((t.value() - 0.5).abs() < 1e-5);
        t.set_target(1.0);
        assert!(t.step(0.1));
        assert!(!t.step(0.1001));
        assert_eq!(t.value(), 1.0);
    }

    #[test]
    fn bob_stays_within_three_px() {
        for i in 0..6 {
            for k in 0..300 {
                let y = jar_bob(k as f64 * 0.05, i);
                assert!((-3.0..=0.0).contains(&y), "{y}");
            }
        }
        assert!((jar_bob(1.5, 0) + 3.0).abs() < 1e-4);
        assert_eq!(jar_bob(0.3, 1), 0.0); // before its delay
    }

    #[test]
    fn spool_turns_once_per_period() {
        assert!(spool_angle(0.0).abs() < 1e-6);
        assert!((spool_angle(1.2) - std::f32::consts::PI).abs() < 1e-4);
    }
}
