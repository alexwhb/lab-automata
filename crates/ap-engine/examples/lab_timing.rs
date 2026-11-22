//! Times Rule-lab runs (400 generations of 64 × 40) for every neighbourhood preset and seed.
//!
//! `cargo run --release -p ap-engine --example lab_timing`

use std::time::Instant;

use ap_engine::{Boundary, LifeRule, NeighbourhoodPreset, RunSpec, Seed, run};

fn main() {
    let mut worst = 0.0f64;
    for preset in NeighbourhoodPreset::ALL {
        for seed in [Seed::RPentomino, Seed::Acorn, Seed::MOCKUP_SOUP] {
            for boundary in Boundary::ALL {
                let spec = RunSpec {
                    rule: LifeRule::LIFE.with_neighbourhood(preset.neighbourhood()),
                    seed,
                    boundary,
                    ..RunSpec::default()
                };
                let reps = 20;
                let start = Instant::now();
                for _ in 0..reps {
                    std::hint::black_box(run(std::hint::black_box(&spec)));
                }
                let ms = start.elapsed().as_secs_f64() * 1000.0 / reps as f64;
                worst = worst.max(ms);
                println!(
                    "{:<12} {:<12} {:<6} {ms:6.2} ms",
                    preset.name(),
                    seed.label(),
                    boundary.key()
                );
            }
        }
    }
    println!("worst: {worst:.2} ms per 400-generation run");
}
