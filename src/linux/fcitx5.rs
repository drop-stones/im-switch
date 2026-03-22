use crate::ImSwitchError;
use fcitx5_dbus::controller::ControllerProxyBlocking;
use fcitx5_dbus::zbus::blocking::Connection;

fn connect() -> Result<ControllerProxyBlocking<'static>, ImSwitchError> {
    let conn = Connection::session()
        .map_err(|e| ImSwitchError::Platform(format!("fcitx5: D-Bus session error: {e}")))?;
    ControllerProxyBlocking::new(&conn)
        .map_err(|e| ImSwitchError::Platform(format!("fcitx5: proxy creation failed: {e}")))
}

pub fn get_input_method() -> Result<String, ImSwitchError> {
    let controller = connect()?;
    controller
        .current_input_method()
        .map_err(|e| ImSwitchError::Platform(format!("fcitx5: failed to get input method: {e}")))
}

pub fn set_input_method(im: &str) -> Result<(), ImSwitchError> {
    let controller = connect()?;
    controller
        .set_current_im(im)
        .map_err(|e| ImSwitchError::Platform(format!("fcitx5: failed to set input method: {e}")))
}
