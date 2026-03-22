use crate::ImSwitchError;
use std::ffi::CString;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::*,
        UI::{
            Input::{Ime::*, KeyboardAndMouse::*},
            WindowsAndMessaging::*,
        },
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

// KL_NAMELENGTH: 8 hex chars + null terminator
const KLID_BUFFER_LEN: usize = 9;

pub fn get_input_method() -> Result<String, ImSwitchError> {
    let mut buffer = [0u8; KLID_BUFFER_LEN];
    unsafe { GetKeyboardLayoutNameA(&mut buffer) }
        .map_err(|e| ImSwitchError::Platform(format!("GetKeyboardLayoutNameA failed: {e}")))?;
    let len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    Ok(String::from_utf8_lossy(&buffer[..len]).to_string())
}

pub fn set_input_method(klid: &str) -> Result<(), ImSwitchError> {
    let cstr = CString::new(klid)
        .map_err(|e| ImSwitchError::Platform(format!("invalid KLID string: {e}")))?;
    let pcstr = PCSTR::from_raw(cstr.as_ptr() as *const u8);
    let hkl = unsafe { LoadKeyboardLayoutA(pcstr, KLF_ACTIVATE) }
        .map_err(|_| ImSwitchError::InputMethodNotFound(klid.to_string()))?;
    if hkl.is_invalid() {
        return Err(ImSwitchError::InputMethodNotFound(klid.to_string()));
    }
    let hwnd = get_foreground_window()?;
    unsafe { PostMessageA(Some(hwnd), WM_INPUTLANGCHANGEREQUEST, WPARAM(0), LPARAM(hkl.0 as isize)) }
        .map_err(|e| ImSwitchError::Platform(format!("PostMessageA failed: {e}")))?;
    Ok(())
}

pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count == 0 {
        return Ok(vec![]);
    }

    let mut hkls = vec![HKL::default(); count as usize];
    unsafe { GetKeyboardLayoutList(Some(&mut hkls)) };

    let klids = hkls
        .iter()
        .map(|hkl| {
            let val = hkl.0 as u32;
            let lo = val & 0xFFFF;
            let hi = (val >> 16) & 0xFFFF;
            if hi == lo {
                // Standard layout: KLID is the language ID
                format!("{lo:08X}")
            } else {
                // IME or special layout: use the full HKL value
                format!("{val:08X}")
            }
        })
        .collect();

    Ok(klids)
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
