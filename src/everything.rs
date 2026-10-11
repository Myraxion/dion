use crate::error::Error;
use std::path::{Path, PathBuf};

/// Finds indexed `descript.ion` paths within a directory tree.
pub fn find_descriptions(directory: &Path, instance: Option<&str>) -> Result<Vec<PathBuf>, Error> {
    #[cfg(windows)]
    {
        windows::find_descriptions(directory, instance)
    }
    #[cfg(not(windows))]
    {
        let _ = (directory, instance);
        Err(Error::new(
            "everything_error",
            "Everything IPC is available only on Windows",
            1,
        ))
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        cell::RefCell,
        ffi::c_void,
        mem, ptr, slice,
        time::{Duration, Instant},
    };

    type Hwnd = *mut c_void;
    type Hinstance = *mut c_void;
    type Lresult = isize;
    type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> Lresult;

    const WM_USER: u32 = 0x0400;
    const WM_COPYDATA: u32 = 0x004A;
    const WM_NCCREATE: u32 = 0x0081;
    const WM_QUIT: u32 = 0x0012;
    const EVERYTHING_IPC_IS_DB_LOADED: usize = 401;
    const EVERYTHING_IPC_COPYDATA_QUERY_W: usize = 2;
    const EVERYTHING_IPC_ALL_RESULTS: u32 = u32::MAX;
    const QUERY_COMPLETE: usize = 0;
    const SMTO_BLOCK: u32 = 0x0001;
    const SMTO_ABORT_IF_HUNG: u32 = 0x0002;
    const SMTO_ERROR_ON_EXIT: u32 = 0x0020;
    const PM_REMOVE: u32 = 0x0001;
    const QS_ALL_INPUT: u32 = 0x04FF;
    const MWMO_INPUT_AVAILABLE: u32 = 0x0004;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;
    const WAIT_FAILED: u32 = u32::MAX;
    const HWND_MESSAGE: Hwnd = -3isize as Hwnd;

    #[repr(C)]
    struct CopyDataStruct {
        data: usize,
        size: u32,
        bytes: *const c_void,
    }

    #[repr(C)]
    struct WindowClass {
        style: u32,
        procedure: Option<WndProc>,
        class_extra: i32,
        window_extra: i32,
        instance: Hinstance,
        icon: *mut c_void,
        cursor: *mut c_void,
        background: *mut c_void,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    struct Message {
        window: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        point_x: i32,
        point_y: i32,
        private: u32,
    }

    thread_local! {
        static QUERY_REPLY: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Hwnd;
        fn SendMessageTimeoutW(
            window: Hwnd,
            message: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> usize;
        fn RegisterClassW(class: *const WindowClass) -> u16;
        fn UnregisterClassW(class_name: *const u16, instance: Hinstance) -> i32;
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: *mut c_void,
            instance: Hinstance,
            parameter: *mut c_void,
        ) -> Hwnd;
        fn DestroyWindow(window: Hwnd) -> i32;
        fn DefWindowProcW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> Lresult;
        fn PeekMessageW(
            message: *mut Message,
            window: Hwnd,
            first: u32,
            last: u32,
            remove: u32,
        ) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> Lresult;
        fn MsgWaitForMultipleObjectsEx(
            count: u32,
            handles: *const Hwnd,
            timeout: u32,
            wake_mask: u32,
            flags: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
        fn GetCurrentProcessId() -> u32;
        fn GetCurrentThreadId() -> u32;
        fn GetLastError() -> u32;
    }

    struct ReplyWindow {
        handle: Hwnd,
        class_name: Vec<u16>,
        instance: Hinstance,
    }

    impl ReplyWindow {
        fn create() -> Result<Self, Error> {
            let class_name: Vec<u16> = format!(
                "DionEverythingReply_{}_{}",
                // SAFETY: These APIs have no pointer or handle arguments.
                unsafe { GetCurrentProcessId() },
                unsafe { GetCurrentThreadId() }
            )
            .encode_utf16()
            .chain(Some(0))
            .collect();
            // SAFETY: A null module name requests this process's module handle.
            let instance = unsafe { GetModuleHandleW(ptr::null()) };
            if instance.is_null() {
                return Err(last_error("Could not get the Dion module handle"));
            }
            let class = WindowClass {
                style: 0,
                procedure: Some(window_proc),
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: ptr::null_mut(),
                cursor: ptr::null_mut(),
                background: ptr::null_mut(),
                menu_name: ptr::null(),
                class_name: class_name.as_ptr(),
            };
            // SAFETY: The class definition and its UTF-16 name remain live through registration.
            if unsafe { RegisterClassW(&class) } == 0 {
                return Err(last_error("Could not register the IPC reply window"));
            }
            // SAFETY: This creates a message-only window owned by the current thread.
            let handle = unsafe {
                CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    ptr::null(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    ptr::null_mut(),
                    instance,
                    ptr::null_mut(),
                )
            };
            if handle.is_null() {
                // SAFETY: Registration succeeded above and no window was created.
                unsafe { UnregisterClassW(class_name.as_ptr(), instance) };
                return Err(last_error("Could not create the IPC reply window"));
            }
            Ok(Self {
                handle,
                class_name,
                instance,
            })
        }
    }

    impl Drop for ReplyWindow {
        fn drop(&mut self) {
            // SAFETY: This object owns the window and class registration.
            unsafe {
                DestroyWindow(self.handle);
                UnregisterClassW(self.class_name.as_ptr(), self.instance);
            }
        }
    }

    pub(super) fn find_descriptions(
        directory: &Path,
        instance: Option<&str>,
    ) -> Result<Vec<PathBuf>, Error> {
        let metadata = std::fs::metadata(directory).map_err(|error| Error::io(error, directory))?;
        if !metadata.is_dir() {
            return Err(Error::new("io_error", "Parent is not a directory", 1).at_file(directory));
        }
        drop(std::fs::read_dir(directory).map_err(|error| Error::io(error, directory))?);
        let deadline = Instant::now() + Duration::from_secs(5);
        let window = find_everything_window(instance, deadline)?;
        ensure_database_loaded(window, deadline)?;
        let query = make_query(directory)?;
        let reply_window = ReplyWindow::create()?;
        QUERY_REPLY.with(|reply| *reply.borrow_mut() = None);
        send_query(window, reply_window.handle, &query, deadline)?;
        let result = wait_for_reply(deadline)?;
        let descriptions = parse_results(&result)?;
        if remaining_millis(deadline).is_none() {
            return Err(ipc_error("Everything IPC timed out"));
        }
        Ok(descriptions)
    }

    fn find_everything_window(instance: Option<&str>, deadline: Instant) -> Result<Hwnd, Error> {
        let classes = match instance {
            Some(instance) => vec![window_class(Some(instance))],
            None => vec![window_class(Some("1.5a")), window_class(None)],
        };
        for class_name in classes {
            if remaining_millis(deadline).is_none() {
                return Err(ipc_error("Everything IPC timed out"));
            }
            // SAFETY: The class name is a live, NUL-terminated UTF-16 string.
            let window = unsafe { FindWindowW(class_name.as_ptr(), ptr::null()) };
            if !window.is_null() {
                return Ok(window);
            }
        }
        Err(ipc_error("Everything instance not found"))
    }

    fn window_class(instance: Option<&str>) -> Vec<u16> {
        let class_name = match instance {
            Some(instance) => format!("EVERYTHING_TASKBAR_NOTIFICATION_({instance})"),
            None => "EVERYTHING_TASKBAR_NOTIFICATION".to_owned(),
        };
        class_name.encode_utf16().chain(Some(0)).collect()
    }

    fn ensure_database_loaded(window: Hwnd, deadline: Instant) -> Result<(), Error> {
        let version = [
            send_window_message(window, 0, 0, deadline, "major version")? as u32,
            send_window_message(window, 1, 0, deadline, "minor version")? as u32,
            send_window_message(window, 2, 0, deadline, "revision")? as u32,
        ];
        if version < [1, 4, 1] {
            return Err(ipc_error("Everything 1.4.1 or later is required"));
        }
        let loaded = send_window_message(
            window,
            EVERYTHING_IPC_IS_DB_LOADED,
            0,
            deadline,
            "database readiness",
        )?;
        if loaded == 0 {
            return Err(ipc_error("Everything database is not ready"));
        }
        Ok(())
    }

    fn send_window_message(
        window: Hwnd,
        command: usize,
        argument: isize,
        deadline: Instant,
        description: &str,
    ) -> Result<usize, Error> {
        let timeout =
            remaining_millis(deadline).ok_or_else(|| ipc_error("Everything IPC timed out"))?;
        let mut result = 0;
        // SAFETY: The destination is the discovered Everything window; scalar message values are valid.
        let sent = unsafe {
            SendMessageTimeoutW(
                window,
                WM_USER,
                command,
                argument,
                SMTO_ABORT_IF_HUNG | SMTO_BLOCK | SMTO_ERROR_ON_EXIT,
                timeout,
                &mut result,
            )
        };
        if sent == 0 {
            return Err(ipc_error(if remaining_millis(deadline).is_none() {
                "Everything IPC timed out"
            } else {
                description
            }));
        }
        Ok(result)
    }

    fn make_query(directory: &Path) -> Result<Vec<u16>, Error> {
        let absolute =
            std::path::absolute(directory).map_err(|error| Error::io(error, directory))?;
        let path = absolute.to_str().ok_or_else(|| {
            Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1)
                .at_file(directory)
        })?;
        let path = path.strip_prefix("\\\\?\\UNC\\").map_or_else(
            || {
                path.strip_prefix("\\\\?\\")
                    .map_or(path.to_owned(), |path| path.to_owned())
            },
            |path| format!("\\\\{path}"),
        );
        let path = path.trim_end_matches(['\\', '/']);
        let search = format!(r#""{path}\" wfn:descript.ion"#);
        Ok(search.encode_utf16().chain(Some(0)).collect())
    }

    fn send_query(
        window: Hwnd,
        reply_window: Hwnd,
        search: &[u16],
        deadline: Instant,
    ) -> Result<(), Error> {
        let timeout =
            remaining_millis(deadline).ok_or_else(|| ipc_error("Everything IPC timed out"))?;
        // Everything's IPC wire format uses 32-bit fields, including window handles, on x64.
        let mut request = Vec::with_capacity(20 + mem::size_of_val(search));
        let fields = [
            reply_window as usize as u32,
            QUERY_COMPLETE as u32,
            0,
            0,
            EVERYTHING_IPC_ALL_RESULTS,
        ];
        for field in fields {
            request.extend_from_slice(&field.to_le_bytes());
        }
        for unit in search {
            request.extend_from_slice(&unit.to_le_bytes());
        }
        let size = u32::try_from(request.len())
            .map_err(|_| ipc_error("Everything IPC query is too large"))?;
        let data = CopyDataStruct {
            data: EVERYTHING_IPC_COPYDATA_QUERY_W,
            size,
            bytes: request.as_ptr().cast(),
        };
        let mut result = 0;
        // SAFETY: Request storage and COPYDATASTRUCT remain live for the synchronous send.
        let sent = unsafe {
            SendMessageTimeoutW(
                window,
                WM_COPYDATA,
                reply_window as usize,
                (&data as *const CopyDataStruct) as isize,
                SMTO_ABORT_IF_HUNG | SMTO_BLOCK | SMTO_ERROR_ON_EXIT,
                timeout,
                &mut result,
            )
        };
        if sent == 0 || result == 0 {
            return Err(ipc_error(if remaining_millis(deadline).is_none() {
                "Everything IPC timed out"
            } else {
                "Everything rejected the IPC query"
            }));
        }
        Ok(())
    }

    fn wait_for_reply(deadline: Instant) -> Result<Vec<u8>, Error> {
        loop {
            if let Some(reply) = QUERY_REPLY.with(|reply| reply.borrow_mut().take()) {
                return Ok(reply);
            }
            let timeout =
                remaining_millis(deadline).ok_or_else(|| ipc_error("Everything IPC timed out"))?;
            // SAFETY: This thread has no wait handles; it waits for input to its own message queue.
            let result = unsafe {
                MsgWaitForMultipleObjectsEx(
                    0,
                    ptr::null(),
                    timeout,
                    QS_ALL_INPUT,
                    MWMO_INPUT_AVAILABLE,
                )
            };
            if result == WAIT_TIMEOUT {
                return Err(ipc_error("Everything IPC timed out"));
            }
            if result == WAIT_FAILED {
                return Err(last_error("Could not wait for the Everything reply"));
            }
            let mut message: Message = unsafe { mem::zeroed() };
            while unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                if message.message == WM_QUIT {
                    return Err(ipc_error("The IPC reply window was closed"));
                }
                // SAFETY: The message was removed from this thread's queue.
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if result != WAIT_OBJECT_0 {
                return Err(ipc_error("Could not wait for the Everything reply"));
            }
        }
    }

    unsafe extern "system" fn window_proc(
        window: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Lresult {
        if message == WM_COPYDATA && lparam != 0 {
            // SAFETY: WM_COPYDATA supplies a valid COPYDATASTRUCT for the duration of this call.
            let data = unsafe { &*(lparam as *const CopyDataStruct) };
            if data.data == QUERY_COMPLETE && data.size >= 28 && !data.bytes.is_null() {
                // SAFETY: Windows guarantees cbData readable bytes during WM_COPYDATA processing.
                let bytes =
                    unsafe { slice::from_raw_parts(data.bytes.cast::<u8>(), data.size as usize) };
                QUERY_REPLY.with(|reply| *reply.borrow_mut() = Some(bytes.to_vec()));
                return 1;
            }
        }
        if message == WM_NCCREATE {
            // SAFETY: Default processing is required for normal window creation.
            return unsafe { DefWindowProcW(window, message, wparam, lparam) };
        }
        // SAFETY: All unhandled messages use the documented default window procedure.
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    fn parse_results(bytes: &[u8]) -> Result<Vec<PathBuf>, Error> {
        if bytes.len() < 28 {
            return Err(ipc_error("Everything returned an invalid result list"));
        }
        let count = read_u32(bytes, 20)? as usize;
        let item_size = 12usize;
        let items_end = 28usize
            .checked_add(count.checked_mul(item_size).ok_or_else(invalid_reply)?)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(invalid_reply)?;
        let mut paths = Vec::with_capacity(count);
        for index in 0..count {
            let item = 28 + index * item_size;
            let name_offset = read_u32(bytes, item + 4)? as usize;
            let path_offset = read_u32(bytes, item + 8)? as usize;
            if name_offset < items_end || path_offset < items_end {
                return Err(invalid_reply());
            }
            let name = read_wide_string(bytes, name_offset)?;
            let path = read_wide_string(bytes, path_offset)?;
            paths.push(PathBuf::from(path).join(name));
        }
        Ok(paths)
    }

    fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
        let value = bytes.get(offset..offset + 4).ok_or_else(invalid_reply)?;
        Ok(u32::from_le_bytes(
            value.try_into().map_err(|_| invalid_reply())?,
        ))
    }

    fn read_wide_string(bytes: &[u8], offset: usize) -> Result<String, Error> {
        if !offset.is_multiple_of(2) || offset >= bytes.len() {
            return Err(invalid_reply());
        }
        let mut wide = Vec::new();
        let mut terminated = false;
        for unit in bytes[offset..].as_chunks::<2>().0 {
            let unit = u16::from_le_bytes([unit[0], unit[1]]);
            if unit == 0 {
                terminated = true;
                break;
            }
            wide.push(unit);
        }
        if !terminated {
            return Err(invalid_reply());
        }
        let mut text = String::new();
        for character in char::decode_utf16(wide) {
            text.push(character.map_err(|_| invalid_reply())?);
        }
        Ok(text)
    }

    fn invalid_reply() -> Error {
        ipc_error("Everything returned an invalid result list")
    }

    fn remaining_millis(deadline: Instant) -> Option<u32> {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        Some(
            u32::try_from(remaining.as_millis())
                .unwrap_or(u32::MAX)
                .max(1),
        )
    }

    fn ipc_error(message: &str) -> Error {
        Error::new("everything_error", message, 1)
    }

    fn last_error(context: &str) -> Error {
        // SAFETY: This API has no arguments and returns the calling thread's last error code.
        let code = unsafe { GetLastError() };
        ipc_error(&format!("{context} (Windows error {code})"))
    }
}
