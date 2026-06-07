mod cli;

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
    /// Run as a loopback daemon, serving commands over TCP
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = cli::ipc::DEFAULT_PORT)]
        port: u16,
        /// Address to bind
        #[arg(long, default_value = "127.0.0.1")]
        addr: String,
    },
    /// Forward a command to a running `im-switch serve` daemon over TCP
    Remote {
        /// Daemon address: host:port, bare port, or bare host (default 127.0.0.1:7691)
        #[arg(long)]
        addr: Option<String>,
        /// The command to forward, e.g. `ime off`, `get`, `set <id>`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
    /// Control IME on/off state (Windows only)
    #[cfg(target_os = "windows")]
    Ime {
        #[command(subcommand)]
        action: cli::windows::ImeAction,
    },
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
        Command::Serve { port, addr } => cli::ipc::run_server(&format!("{addr}:{port}")),
        Command::Remote { addr, command } => {
            let target = addr
                .as_deref()
                .map(cli::client::resolve_addr)
                .unwrap_or_else(cli::client::default_addr);
            std::process::exit(cli::client::forward(&target, &command));
        }
        #[cfg(target_os = "windows")]
        Command::Ime { action } => cli::windows::handle_ime(action),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
