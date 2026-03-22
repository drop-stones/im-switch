mod error;

pub use error::ImSwitchError;

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

mod platform {
    use super::ImSwitchError;

    pub fn get_input_method() -> Result<String, ImSwitchError> {
        Err(ImSwitchError::UnsupportedPlatform)
    }

    pub fn set_input_method(_im: &str) -> Result<(), ImSwitchError> {
        Err(ImSwitchError::UnsupportedPlatform)
    }
}
