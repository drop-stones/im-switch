use clap::Subcommand;
use im_switch::{get_ime_state, set_ime_state, toggle_ime_state, ImSwitchError};

#[derive(Subcommand)]
pub enum ImeAction {
    /// Get the current IME state
    Get,
    /// Turn on the IME
    On,
    /// Turn off the IME
    Off,
    /// Toggle the IME state
    Toggle,
}

pub fn handle_ime(action: ImeAction) -> Result<(), ImSwitchError> {
    match action {
        ImeAction::Get => {
            let state = if get_ime_state()? { "on" } else { "off" };
            println!("{state}");
            Ok(())
        }
        ImeAction::On => set_ime_state(true),
        ImeAction::Off => set_ime_state(false),
        ImeAction::Toggle => {
            let new_state = toggle_ime_state()?;
            let state = if new_state { "on" } else { "off" };
            println!("{state}");
            Ok(())
        }
    }
}
