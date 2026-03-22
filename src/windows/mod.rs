use crate::ImSwitchError;
use windows::Win32::{
    Foundation::*,
    UI::{
        Input::{Ime::*, KeyboardAndMouse::*},
        WindowsAndMessaging::*,
    },
};

const IMC_GETOPENSTATUS: WPARAM = WPARAM(5);
const IMC_SETOPENSTATUS: WPARAM = WPARAM(6);

fn get_foreground_window() -> Result<HWND, ImSwitchError> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Err(ImSwitchError::Platform(
            "GetForegroundWindow failed".to_string(),
        ));
    }
    Ok(hwnd)
}

fn get_ime_window() -> Result<HWND, ImSwitchError> {
    let hwnd = get_foreground_window()?;
    let ime = unsafe { ImmGetDefaultIMEWnd(hwnd) };
    if ime.is_invalid() {
        return Err(ImSwitchError::Platform(
            "ImmGetDefaultIMEWnd failed".to_string(),
        ));
    }
    Ok(ime)
}

// --- Keyboard layout (KLID) based API ---

pub fn get_input_method() -> Result<String, ImSwitchError> {
    let layout = unsafe { GetKeyboardLayout(0) };
    // KLID is derived from the low word of the layout handle
    let klid = (layout.0 as u32) & 0xFFFF;
    Ok(format!("{:08X}", klid))
}

pub fn set_input_method(klid: &str) -> Result<(), ImSwitchError> {
    let layout = unsafe { LoadKeyboardLayoutA(&PCSTR::from_raw(klid.as_ptr()), KLF_ACTIVATE) };
    if layout.is_invalid() {
        return Err(ImSwitchError::InputMethodNotFound(klid.to_string()));
    }
    Ok(())
}

// --- IME on/off API ---

pub fn get_ime_state() -> Result<bool, ImSwitchError> {
    let ime = get_ime_window()?;
    let status = unsafe { SendMessageA(ime, WM_IME_CONTROL, IMC_GETOPENSTATUS, LPARAM(0)) };
    Ok(status.0 != 0)
}

pub fn set_ime_state(enabled: bool) -> Result<(), ImSwitchError> {
    let ime = get_ime_window()?;
    let lparam = if enabled { LPARAM(1) } else { LPARAM(0) };
    unsafe { SendMessageA(ime, WM_IME_CONTROL, IMC_SETOPENSTATUS, lparam) };
    Ok(())
}
