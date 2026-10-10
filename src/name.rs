use std::cmp::Ordering;

/// A name compared by Windows ordinal rules, independently of display sorting.
pub struct Name(Vec<u16>);

impl Name {
    pub fn new(name: &str) -> Self {
        Self(name.encode_utf16().collect())
    }

    pub fn matches(&self, name: &str) -> bool {
        *self == Self::new(name)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        ordinal_cmp(&self.0, &other.0)
    }
}

#[cfg(windows)]
fn ordinal_cmp(a: &[u16], b: &[u16]) -> Ordering {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CompareStringOrdinal(
            a: *const u16,
            a_len: i32,
            b: *const u16,
            b_len: i32,
            ignore_case: i32,
        ) -> i32;
    }
    // SAFETY: The live UTF-16 buffers cover their explicit lengths, including
    // embedded NUL. Names come from <=4096-byte records or Windows paths/argv,
    // so lengths fit i32. TRUE is exactly 1; all API parameters are valid.
    let result =
        unsafe { CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1) };
    // The only documented failure is invalid parameters, excluded above.
    debug_assert!((1..=3).contains(&result));
    result.cmp(&2)
}

#[cfg(not(windows))]
fn ordinal_cmp(a: &[u16], b: &[u16]) -> Ordering {
    // Only Windows is supported; retain a buildable non-Windows stub.
    a.cmp(b)
}
