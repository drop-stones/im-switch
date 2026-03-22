use crate::ImSwitchError;
use core_foundation::{
    array::CFArray,
    base::{CFTypeID, FromVoid, OSStatus, TCFType, ToVoid},
    declare_TCFType,
    dictionary::CFDictionary,
    impl_TCFType,
    string::{CFString, CFStringRef},
};
use std::ffi::c_void;

// --- Carbon TIS FFI bindings ---

type CFDataRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFArrayRef = *const core_foundation::array::__CFArray;

#[derive(Debug)]
#[repr(transparent)]
pub struct __TISInputSource(c_void);
type TISInputSourceRef = *const __TISInputSource;

declare_TCFType!(TISInputSource, TISInputSourceRef);
impl_TCFType!(TISInputSource, TISInputSourceRef, TISInputSourceGetTypeID);

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISInputSourceGetTypeID() -> CFTypeID;
    fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    fn TISGetInputSourceProperty(source: TISInputSourceRef, property_key: CFStringRef)
        -> CFDataRef;
    fn TISCreateInputSourceList(
        properties: CFDictionaryRef,
        include_all_installed: bool,
    ) -> CFArrayRef;
    fn TISSelectInputSource(source: TISInputSourceRef) -> OSStatus;

    static kTISPropertyInputSourceID: CFStringRef;
}

// --- Public API ---

pub fn get_input_method() -> Result<String, ImSwitchError> {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return Err(ImSwitchError::Platform(
                "macOS: TISCopyCurrentKeyboardInputSource returned null".to_string(),
            ));
        }
        let source_id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) as CFStringRef;
        if source_id.is_null() {
            return Err(ImSwitchError::Platform(
                "macOS: failed to get input source ID".to_string(),
            ));
        }
        Ok(CFString::wrap_under_get_rule(source_id).to_string())
    }
}

pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    unsafe {
        let sources = CFArray::<TISInputSource>::wrap_under_get_rule(TISCreateInputSourceList(
            std::ptr::null(),
            false,
        ));
        let mut result = Vec::with_capacity(sources.len() as usize);
        for i in 0..sources.len() {
            let source = sources.get(i).ok_or_else(|| {
                ImSwitchError::Platform("macOS: failed to get input source from list".to_string())
            })?;
            let source_id =
                TISGetInputSourceProperty(source.as_concrete_TypeRef(), kTISPropertyInputSourceID)
                    as CFStringRef;
            if !source_id.is_null() {
                result.push(CFString::wrap_under_get_rule(source_id).to_string());
            }
        }
        Ok(result)
    }
}

pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    // Skip if already set
    if get_input_method()? == im {
        return Ok(());
    }

    unsafe {
        let filter = CFDictionary::from_CFType_pairs(&[(
            CFString::from_void(kTISPropertyInputSourceID.cast()).clone(),
            CFString::new(im),
        )]);
        let sources = CFArray::<TISInputSource>::wrap_under_get_rule(TISCreateInputSourceList(
            filter.to_untyped().to_void().cast(),
            false,
        ));
        let source = sources
            .get(0)
            .ok_or_else(|| ImSwitchError::InputMethodNotFound(im.to_string()))?;
        let status = TISSelectInputSource(source.as_concrete_TypeRef());
        if status != 0 {
            return Err(ImSwitchError::Platform(format!(
                "macOS: TISSelectInputSource failed with status {status}"
            )));
        }
    }
    Ok(())
}
