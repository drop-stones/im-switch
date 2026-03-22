use clap::{Parser, Subcommand};
use im_switch::{get_im, set_im};

#[derive(Parser)]
#[command(author, version, about = "Cross-platform input method switcher")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Get the current input method
    Get,
    /// Set the input method
    Set {
        /// Input method identifier to set
        im: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Get => get_im().map(|im| println!("{im}")),
        Command::Set { ref im } => set_im(im),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
