use crate::ImSwitchError;

pub fn get_input_method() -> Result<String, ImSwitchError> {
    Err(ImSwitchError::UnsupportedPlatform)
}

pub fn set_input_method(_im: &str) -> Result<(), ImSwitchError> {
    Err(ImSwitchError::UnsupportedPlatform)
}

pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    Err(ImSwitchError::UnsupportedPlatform)
}
