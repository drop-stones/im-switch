use clap::{Parser, Subcommand};
use im_switch::{get_input_method, list_input_methods, set_input_method};

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
    /// List available input methods
    List,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Get => get_input_method().map(|im| println!("{im}")),
        Command::Set { ref im } => set_input_method(im),
        Command::List => list_input_methods().map(|methods| {
            for method in &methods {
                println!("{method}");
            }
        }),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
