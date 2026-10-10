#[cfg(windows)]
pub struct ConsoleModeGuard {
    handle: *mut std::ffi::c_void,
    original_mode: u32,
}

#[cfg(windows)]
const STD_OUTPUT_HANDLE: u32 = (-11_i32) as u32;
#[cfg(windows)]
const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
#[cfg(windows)]
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
#[cfg(windows)]
const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1_isize as *mut _;

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    #[link_name = "GetStdHandle"]
    fn get_std_handle(n_std_handle: u32) -> *mut std::ffi::c_void;
    #[link_name = "GetConsoleMode"]
    fn get_console_mode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    #[link_name = "SetConsoleMode"]
    fn set_console_mode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
}

#[cfg(windows)]
impl Drop for ConsoleModeGuard {
    fn drop(&mut self) {
        // SAFETY: The handle was returned for stdout and remains valid for the
        // duration of this guard; restoring its captured mode does not retain it.
        unsafe {
            let _ = set_console_mode(self.handle, self.original_mode);
        }
    }
}

#[cfg(windows)]
pub fn enable_virtual_terminal_processing() -> Option<ConsoleModeGuard> {
    // SAFETY: GetStdHandle has no preconditions; the returned handle is validated
    // before it is passed to either console API.
    let handle = unsafe { get_std_handle(STD_OUTPUT_HANDLE) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut original_mode = 0;
    // SAFETY: `original_mode` is writable and `handle` is a candidate stdout handle.
    if unsafe { get_console_mode(handle, &mut original_mode) } == 0 {
        return None;
    }
    let color_mode = original_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    // SAFETY: `handle` is a console handle and color_mode only adds documented flags.
    if unsafe { set_console_mode(handle, color_mode) } == 0 {
        return None;
    }
    Some(ConsoleModeGuard {
        handle,
        original_mode,
    })
}

#[cfg(not(windows))]
pub struct ConsoleModeGuard;

#[cfg(not(windows))]
pub fn enable_virtual_terminal_processing() -> Option<ConsoleModeGuard> {
    Some(ConsoleModeGuard)
}
