//! Controlled external IPC peer, using the public Everything LISTW wire layout.
//! https://www.voidtools.com/support/everything/sdk/ipc/
use std::{ffi::c_void, fs, mem, ptr, slice, sync::OnceLock};

type Hwnd = *mut c_void;
type Procedure = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[repr(C)]
struct CopyData {
    tag: usize,
    len: u32,
    bytes: *const c_void,
}
#[repr(C)]
struct Class {
    style: u32,
    procedure: Option<Procedure>,
    class_extra: i32,
    window_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu: *const u16,
    name: *const u16,
}
#[repr(C)]
struct Message {
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const Class) -> u16;
    fn FindWindowW(class: *const u16, name: *const u16) -> Hwnd;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *mut c_void,
        instance: *mut c_void,
        param: *mut c_void,
    ) -> Hwnd;
    fn DefWindowProcW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn PostMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
    fn SendMessageTimeoutW(
        window: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
    fn GetMessageW(message: *mut Message, window: Hwnd, first: u32, last: u32) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
}

struct Reply {
    mode: String,
    bytes: Vec<u8>,
    query: String,
}
static REPLY: OnceLock<Reply> = OnceLock::new();
const SEND_REPLY: u32 = 0x8001;

unsafe extern "system" fn procedure(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let reply = REPLY.get().unwrap();
    if message == 0x0400 {
        return match wparam {
            0 => 1,
            1 => {
                if reply.mode == "old" {
                    3
                } else {
                    4
                }
            }
            2 => 1,
            401 => isize::from(reply.mode != "unready"),
            _ => 0,
        };
    }
    if message == 0x004A {
        // SAFETY: WM_COPYDATA supplies this structure and buffer for the call duration.
        let data = unsafe { &*(lparam as *const CopyData) };
        assert_eq!(data.tag, 2);
        // SAFETY: Windows marshals the declared WM_COPYDATA buffer for this callback.
        let request = unsafe { slice::from_raw_parts(data.bytes.cast::<u8>(), data.len as usize) };
        assert_eq!(
            &request[8..20],
            &[0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]
        );
        let wide = request[20..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect::<Vec<_>>();
        fs::write(&reply.query, String::from_utf16(&wide).unwrap()).unwrap();
        if reply.mode == "reject" {
            return 0;
        }
        if reply.mode != "timeout" {
            let target = u32::from_le_bytes(request[0..4].try_into().unwrap());
            let tag = u32::from_le_bytes(request[4..8].try_into().unwrap());
            // SAFETY: This is our live window; both posted parameters are scalar wire fields.
            // Posting lets the query send return before the synchronous reply.
            unsafe { PostMessageW(window, SEND_REPLY, target as usize, tag as isize) };
        }
        return 1;
    }
    if message == SEND_REPLY {
        let data = CopyData {
            tag: lparam as usize,
            len: reply.bytes.len() as u32,
            bytes: reply.bytes.as_ptr().cast(),
        };
        let mut result = 0;
        // SAFETY: The response and structure stay live throughout this bounded send.
        unsafe {
            SendMessageTimeoutW(
                wparam as Hwnd,
                0x004A,
                window as usize,
                (&data as *const CopyData) as isize,
                0x0002,
                10_000,
                &mut result,
            )
        };
        return 0;
    }
    // SAFETY: Windows supplied these arguments; unhandled messages use default processing.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    REPLY
        .set(Reply {
            mode: args[4].clone(),
            bytes: fs::read(&args[1]).unwrap(),
            query: args[3].clone(),
        })
        .ok()
        .unwrap();
    let name = if args[0].is_empty() {
        "EVERYTHING_TASKBAR_NOTIFICATION".to_owned()
    } else {
        format!("EVERYTHING_TASKBAR_NOTIFICATION_({})", args[0])
    };
    let name = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    // Never shadow an already-running instance owned by the user or another test.
    // SAFETY: The class name is live and NUL-terminated; the null window name is optional.
    if !unsafe { FindWindowW(name.as_ptr(), ptr::null()) }.is_null() {
        fs::write(&args[2], "occupied").unwrap();
        return;
    }
    // SAFETY: A null name requests the current process module.
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class = Class {
        style: 0,
        procedure: Some(procedure),
        class_extra: 0,
        window_extra: 0,
        instance,
        icon: ptr::null_mut(),
        cursor: ptr::null_mut(),
        background: ptr::null_mut(),
        menu: ptr::null(),
        name: name.as_ptr(),
    };
    // SAFETY: The structure and class name remain live; the procedure has the Windows ABI.
    assert_ne!(unsafe { RegisterClassW(&class) }, 0);
    // SAFETY: The class is registered in this module; all optional arguments are null.
    let window = unsafe {
        CreateWindowExW(
            0,
            name.as_ptr(),
            ptr::null(),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        )
    };
    assert!(!window.is_null());
    fs::write(&args[2], "ready").unwrap();
    // SAFETY: This C structure consists only of scalars and nullable pointers.
    let mut message: Message = unsafe { mem::zeroed() };
    // SAFETY: The message buffer is writable and null selects this thread's queue.
    while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {
        // SAFETY: GetMessage initialized the complete message above.
        unsafe { DispatchMessageW(&message) };
    }
}
