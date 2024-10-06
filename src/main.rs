pub use ordinary::auth;
pub use ordinary::optimizer;
pub use ordinary::paths;
pub use ordinary::storage;

use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Command {
    Start,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_enum)]
    command: Command,

    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

use ordinary::app::start;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    match args.command {
        Command::Start => start()?,
    }

    Ok(())
}
