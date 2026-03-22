use crate::ImSwitchError;
use std::process::Command;

pub fn get_input_method() -> Result<String, ImSwitchError> {
    let output = Command::new("ibus")
        .arg("engine")
        .output()
        .map_err(|e| ImSwitchError::Platform(format!("ibus: failed to run command: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ImSwitchError::Platform(format!(
            "ibus: engine query failed: {stderr}"
        )));
    }

    let im = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(im)
}

pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    let output = Command::new("ibus")
        .args(["engine", im])
        .output()
        .map_err(|e| ImSwitchError::Platform(format!("ibus: failed to run command: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ImSwitchError::Platform(format!(
            "ibus: failed to set engine: {stderr}"
        )));
    }

    Ok(())
}

pub fn list_input_methods() -> Result<Vec<String>, ImSwitchError> {
    let output = Command::new("ibus")
        .args(["list-engine"])
        .output()
        .map_err(|e| ImSwitchError::Platform(format!("ibus: failed to run command: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ImSwitchError::Platform(format!(
            "ibus: list-engine failed: {stderr}"
        )));
    }

    // Each line looks like "  xkb:us::eng" (indented with spaces)
    let engines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    Ok(engines)
}
