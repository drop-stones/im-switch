// --- Module declarations ---

mod error;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod unsupported;

// --- Re-exports and platform alias ---

pub use error::ImSwitchError;

#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use unsupported as platform;

// --- Cross-platform API ---

/// Returns the current input method identifier.
///
/// The returned value is platform-dependent:
/// - Windows: Keyboard layout ID (KLID), e.g., `"00000409"`
/// - macOS: Input source identifier, e.g., `"com.apple.keylayout.ABC"`
/// - Linux: Input method name (IM framework-dependent)
pub fn get_input_method() -> Result<String, ImSwitchError> {
    platform::get_input_method()
}

/// Sets the input method to the specified identifier.
///
/// The identifier format is platform-dependent (see [`get_input_method`]).
pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    platform::set_input_method(im)
}

/// Returns a list of available input method identifiers.
pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    platform::list_input_methods()
}

// --- Windows-only API ---

/// Returns the current IME on/off state (Windows only).
#[cfg(target_os = "windows")]
pub fn get_ime_state() -> Result<bool, ImSwitchError> {
    platform::get_ime_state()
}

/// Sets the IME on/off state (Windows only).
#[cfg(target_os = "windows")]
pub fn set_ime_state(enabled: bool) -> Result<(), ImSwitchError> {
    platform::set_ime_state(enabled)
}

/// Toggles the IME on/off state and returns the new state (Windows only).
#[cfg(target_os = "windows")]
pub fn toggle_ime_state() -> Result<bool, ImSwitchError> {
    let current = get_ime_state()?;
    let new_state = !current;
    set_ime_state(new_state)?;
    Ok(new_state)
}
