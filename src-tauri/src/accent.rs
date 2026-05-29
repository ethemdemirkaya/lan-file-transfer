//! Read the user's Windows accent color from the registry so the UI can
//! adopt it as its Fluent brand colour. Reading the registry directly is
//! enough — we don't need to subscribe to `WM_DWMCOLORIZATIONCOLORCHANGED`
//! because the frontend polls on demand and on settings open.

#[cfg(target_os = "windows")]
pub fn read_accent_hex() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hk = RegKey::predef(HKEY_CURRENT_USER);
    let dwm = hk.open_subkey("Software\\Microsoft\\Windows\\DWM").ok()?;
    // AccentColor is stored as ABGR (DWM is weird about this).
    let value: u32 = dwm.get_value("AccentColor").ok()?;
    let r = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let b = ((value >> 16) & 0xFF) as u8;
    Some(format!("#{:02X}{:02X}{:02X}", r, g, b))
}

#[cfg(not(target_os = "windows"))]
pub fn read_accent_hex() -> Option<String> {
    None
}
