use im_switch::{get_input_method, list_input_methods, set_input_method};
use serial_test::serial;

#[test]
#[serial]
#[ignore]
fn get_input_method_returns_non_empty_string() {
    let im = get_input_method().unwrap();
    assert!(!im.is_empty(), "input method should not be empty");
}

#[test]
#[serial]
#[ignore]
fn list_input_methods_returns_non_empty_list() {
    let methods = list_input_methods().unwrap();
    assert!(!methods.is_empty(), "should have at least one input method");
}

#[test]
#[serial]
#[ignore]
fn current_input_method_is_in_list() {
    let current = get_input_method().unwrap();
    let methods = list_input_methods().unwrap();
    assert!(
        methods.contains(&current),
        "current IM '{current}' should be in the list: {methods:?}"
    );
}

#[test]
#[serial]
#[ignore]
fn set_then_get_roundtrip() {
    let original = get_input_method().unwrap();

    // Set to the same value (should always succeed)
    set_input_method(&original).unwrap();

    let after = get_input_method().unwrap();
    assert_eq!(original, after, "input method should not change after set");
}

#[test]
#[serial]
#[ignore]
fn list_contains_no_empty_entries() {
    let methods = list_input_methods().unwrap();
    for method in &methods {
        assert!(!method.is_empty(), "list should not contain empty strings");
    }
}
