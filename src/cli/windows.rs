use clap::Subcommand;
use im_switch::{get_ime_state, set_ime_state, toggle_ime_state, ImSwitchError};

#[derive(Subcommand)]
pub enum ImeAction {
    /// Get the current IME state
    Get,
    /// Enable the IME
    Enable,
    /// Disable the IME
    Disable,
    /// Toggle the IME state
    Toggle,
}

pub fn handle_ime(action: ImeAction) -> Result<(), ImSwitchError> {
    match action {
        ImeAction::Get => {
            let state = if get_ime_state()? { "enabled" } else { "disabled" };
            println!("{state}");
            Ok(())
        }
        ImeAction::Enable => set_ime_state(true),
        ImeAction::Disable => set_ime_state(false),
        ImeAction::Toggle => {
            let new_state = toggle_ime_state()?;
            let state = if new_state { "enabled" } else { "disabled" };
            println!("{state}");
            Ok(())
        }
    }
}
