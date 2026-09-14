pub fn accessibility_trusted(_: bool) -> bool {
    true
}
pub fn title(_: i32) -> Option<String> {
    None
}
#[cfg(not(target_os = "windows"))]
pub fn idle_seconds() -> f64 {
    0.0
}
#[cfg(target_os = "windows")]
pub fn idle_seconds() -> f64 {
    #[repr(C)]
    struct LastInputInfo {
        size: u32,
        time: u32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetTickCount() -> u32;
    }
    let mut info = LastInputInfo {
        size: std::mem::size_of::<LastInputInfo>() as u32,
        time: 0,
    };
    unsafe {
        if GetLastInputInfo(&mut info) != 0 {
            f64::from(GetTickCount().wrapping_sub(info.time)) / 1000.0
        } else {
            0.0
        }
    }
}
