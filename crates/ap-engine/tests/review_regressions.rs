//! Regressions for the independent review's engine findings (each test names its finding).

use ap_engine::RuleScript;
use std::time::{Duration, Instant};

/// Parses `text` and checks it is valid and took well under a frame budget even in the test
/// profile (the release bound would be < 50 ms).
fn parses_quickly(text: &str) {
    let t = Instant::now();
    let parsed = RuleScript::parse(text);
    let took = t.elapsed();
    assert!(parsed.is_ok(), "{parsed:?}");
    assert!(
        took < Duration::from_millis(200),
        "{} chars took {took:?}",
        text.chars().count()
    );
}

/// Review finding 8: the rule-script tokenizer looked ahead over the rest of a whitespace run
/// for every whitespace character, so a 100 k-space line took ~2 s to parse (release). It is
/// linear now.
#[test]
fn long_whitespace_runs_parse_in_linear_time() {
    let text = format!(
        ".rule{}life_like B3/S23\n.neigh moore\n.grid 64x40\n.seed rpent\n.run gens=400\n.end",
        " ".repeat(100_000)
    );
    assert!(text.chars().count() > 100_000);
    parses_quickly(&text);
    // spaces before `=` (glued to the key) and inside parentheses take the other branches
    let text = format!(
        ".rule life_like B3/S23\n.neigh moore\n.grid 64x40 boundary{}=torus\n.seed rpent @ ({}32,20)\n.run gens=400",
        " ".repeat(50_000),
        " ".repeat(50_000)
    );
    parses_quickly(&text);
}

/// Review finding 10: superseded Rule-lab runs kept computing (10 quick edits = 10 threads ×
/// ~0.6 s). `run_cancellable` checks a flag between generations: set, it stops within a
/// generation and returns `None`; never set, it is exactly `run`.
#[test]
fn runs_can_be_cancelled_between_generations() {
    use ap_engine::analysis::{self, RunSpec, Seed};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let small = RunSpec::default();
    let never = AtomicBool::new(false);
    assert_eq!(
        analysis::run_cancellable(&small, &never),
        Some(analysis::run(&small))
    );
    assert_eq!(
        analysis::run_cancellable(&small, &AtomicBool::new(true)),
        None
    );
    // a big run cancelled from another thread stops long before it would finish
    let big = RunSpec {
        w: 256,
        h: 256,
        gens: 2000,
        seed: Seed::MOCKUP_SOUP,
        ..RunSpec::default()
    };
    let flag = Arc::new(AtomicBool::new(false));
    let worker = {
        let flag = Arc::clone(&flag);
        std::thread::spawn(move || {
            let t = Instant::now();
            (analysis::run_cancellable(&big, &flag), t.elapsed())
        })
    };
    std::thread::sleep(Duration::from_millis(20));
    flag.store(true, Ordering::Relaxed);
    let (out, took) = worker.join().unwrap();
    assert!(out.is_none(), "a cancelled run returns nothing");
    assert!(took < Duration::from_millis(150), "stopped after {took:?}");
}
