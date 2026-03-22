mod error;

pub use error::ImSwitchError;

/// Returns the current input method identifier.
///
/// The returned value is platform-dependent:
/// - Windows: `"on"` or `"off"` (IME state)
/// - macOS: Input source identifier (e.g., `"com.apple.keylayout.ABC"`)
/// - Linux: Input method name (IM framework-dependent)
pub fn get_im() -> Result<String, ImSwitchError> {
    platform::get_im()
}

/// Sets the input method to the specified identifier.
///
/// The identifier format is platform-dependent (see [`get_im`]).
pub fn set_im(im: &str) -> Result<(), ImSwitchError> {
    platform::set_im(im)
}

mod platform {
    use super::ImSwitchError;

    pub fn get_im() -> Result<String, ImSwitchError> {
        Err(ImSwitchError::UnsupportedPlatform)
    }

    pub fn set_im(_im: &str) -> Result<(), ImSwitchError> {
        Err(ImSwitchError::UnsupportedPlatform)
    }
}
