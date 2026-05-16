use std::env;
use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

pub fn set_autostart(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let app_name = w!("WinIsland");
    let sub_key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

    if enabled {
        let exe_path = env::current_exe()?;
        let exe_path_str = exe_path.to_str().ok_or("Invalid exe path")?;
        let exe_path_wide: Vec<u16> = exe_path_str
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut hkey = HKEY::default();
            let res = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                sub_key,
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            );

            if res.is_ok() {
                let _ = RegSetValueExW(
                    hkey,
                    app_name,
                    0,
                    REG_SZ,
                    Some(std::slice::from_raw_parts(
                        exe_path_wide.as_ptr() as *const u8,
                        exe_path_wide.len() * 2,
                    )),
                );
                let _ = RegCloseKey(hkey);
            }
        }
    } else {
        unsafe {
            let mut hkey = HKEY::default();
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                sub_key,
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
            .is_ok()
            {
                let _ = RegDeleteValueW(hkey, app_name);
                let _ = RegCloseKey(hkey);
            }
        }
    }
    Ok(())
}
