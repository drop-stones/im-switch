use crate::ImSwitchError;
use objc2::rc::Retained;
use objc2_app_kit::{NSTextInputContext, NSTextInputSourceIdentifier};
use objc2_foundation::{MainThreadMarker, NSString};
use std::{ffi::CStr, ops::Deref};

fn nsstring_to_string(nsstr: &NSString) -> Result<String, ImSwitchError> {
    let cstr = unsafe { CStr::from_ptr(nsstr.UTF8String()) };
    cstr.to_str()
        .map(|s| s.to_owned())
        .map_err(|e| ImSwitchError::Platform(format!("macOS: UTF-8 conversion error: {e}")))
}

fn create_input_context() -> Result<Retained<NSTextInputContext>, ImSwitchError> {
    let marker = MainThreadMarker::new().ok_or_else(|| {
        ImSwitchError::Platform("macOS: must be called from the main thread".to_string())
    })?;
    Ok(unsafe { NSTextInputContext::new(marker) })
}

fn get_keyboard_input_sources(ctx: &NSTextInputContext) -> Option<Vec<Retained<NSString>>> {
    ctx.keyboardInputSources().map(|v| v.to_vec())
}

fn is_input_method_available(im: &str) -> Result<bool, ImSwitchError> {
    let ctx = create_input_context()?;
    let sources = get_keyboard_input_sources(&ctx).ok_or_else(|| {
        ImSwitchError::Platform("macOS: failed to get keyboard input sources".to_string())
    })?;
    for source in &sources {
        if nsstring_to_string(source)? == im {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn get_input_method() -> Result<String, ImSwitchError> {
    let ctx = create_input_context()?;
    let source = ctx.selectedKeyboardInputSource().ok_or_else(|| {
        ImSwitchError::Platform("macOS: failed to get selected input source".to_string())
    })?;
    nsstring_to_string(source.deref())
}

pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    if !is_input_method_available(im)? {
        return Err(ImSwitchError::InputMethodNotFound(im.to_string()));
    }

    // Skip if already set
    if get_input_method()? == im {
        return Ok(());
    }

    let ctx = create_input_context()?;
    let id: Retained<NSTextInputSourceIdentifier> = NSString::from_str(im);
    ctx.setSelectedKeyboardInputSource(Some(id.deref()));
    Ok(())
}
