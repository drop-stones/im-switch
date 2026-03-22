mod fcitx5;
mod ibus;

use crate::ImSwitchError;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
enum ImFramework {
    Fcitx5,
    Ibus,
}

static DETECTED_FRAMEWORK: OnceLock<Option<ImFramework>> = OnceLock::new();

fn detect_framework() -> Option<ImFramework> {
    // Check XMODIFIERS environment variable (e.g., "@im=fcitx" or "@im=ibus")
    if let Ok(xmod) = std::env::var("XMODIFIERS") {
        if xmod.contains("fcitx") {
            return Some(ImFramework::Fcitx5);
        }
        if xmod.contains("ibus") {
            return Some(ImFramework::Ibus);
        }
    }

    // Check if fcitx5 D-Bus service is available
    if fcitx5::get_input_method().is_ok() {
        return Some(ImFramework::Fcitx5);
    }

    // Check if ibus command is available
    if ibus::get_input_method().is_ok() {
        return Some(ImFramework::Ibus);
    }

    None
}

fn get_framework() -> Result<ImFramework, ImSwitchError> {
    DETECTED_FRAMEWORK
        .get_or_init(detect_framework)
        .ok_or_else(|| {
            ImSwitchError::Platform(
                "no supported input method framework detected (fcitx5 or ibus)".to_string(),
            )
        })
}

pub fn get_input_method() -> Result<String, ImSwitchError> {
    match get_framework()? {
        ImFramework::Fcitx5 => fcitx5::get_input_method(),
        ImFramework::Ibus => ibus::get_input_method(),
    }
}

pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    match get_framework()? {
        ImFramework::Fcitx5 => fcitx5::set_input_method(im),
        ImFramework::Ibus => ibus::set_input_method(im),
    }
}

pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    match get_framework()? {
        ImFramework::Fcitx5 => fcitx5::list_input_methods(),
        ImFramework::Ibus => ibus::list_input_methods(),
    }
}
