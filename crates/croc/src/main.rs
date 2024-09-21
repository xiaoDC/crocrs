use clap::Parser;

mod cli;
mod comm;
mod croc;
mod crypt;
mod model;
mod tcp;
mod utils;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let cli = cli::App::parse();
    let _rst = cli.run();
    Ok(())
}
