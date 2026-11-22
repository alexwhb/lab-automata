//! Automata Playground desktop app entry point.

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("automata_playground=info,warn"),
    )
    .init();
    automata_playground::run()
}
