//! Automata Playground desktop app entry point.

fn main() -> eframe::Result<()> {
    automata_playground::STARTED.get_or_init(std::time::Instant::now);
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("automata_playground=info,warn"),
    )
    .init();
    // `automata-playground --smoke <seconds> [--headless]`: start-up + run measurement
    let args: Vec<String> = std::env::args().collect();
    if let Some(smoke) = automata_playground::smoke::SmokeArgs::parse(&args) {
        match smoke {
            Ok(a) => std::process::exit(automata_playground::smoke::main(a)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    }
    automata_playground::run()
}
