//! The C ABI the browser calls (no wasm-bindgen). Pixels are passed through linear memory:
//! JS asks for a buffer with [`ac_alloc`], writes RGBA8 into it, calls [`ac_correct`] and reads
//! the corrected pixels back from the same buffer.

use std::cell::RefCell;

use crate::render::Path;

thread_local! {
    static REPORT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A buffer of `len` bytes for JS to fill; null when memory is short.
#[unsafe(no_mangle)]
pub extern "C" fn ac_alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::new();
    if v.try_reserve_exact(len).is_err() {
        return std::ptr::null_mut();
    }
    v.resize(len, 0);
    let mut b = v.into_boxed_slice();
    let p = b.as_mut_ptr();
    std::mem::forget(b);
    p
}

/// Frees a buffer from [`ac_alloc`].
///
/// # Safety
/// `ptr` and `len` must come from one [`ac_alloc`] call, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ac_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
    }
}

/// Bytes the full path needs at peak for a `w` x `h` photo (measured: about 82 per pixel).
#[unsafe(no_mangle)]
pub extern "C" fn ac_full_bytes(w: u32, h: u32) -> f64 {
    w as f64 * h as f64 * 90.0 + 16.0 * 1024.0 * 1024.0
}

/// Corrects the RGBA8 photo at `ptr` in place. `path`: 0 full, 1 banded.
/// Returns 0 on success, -1 for a bad size or pointer, -2 for an unknown path.
///
/// # Safety
/// `ptr` must point to `w * h * 4` bytes from [`ac_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ac_correct(ptr: *mut u8, w: u32, h: u32, path: u32) -> i32 {
    let (w, h) = (w as usize, h as usize);
    if ptr.is_null() || w == 0 || h == 0 {
        return -1;
    }
    let path = match path {
        0 => Path::Full,
        1 => Path::Banded,
        _ => return -2,
    };
    let rgba = unsafe { std::slice::from_raw_parts_mut(ptr, w * h * 4) };
    let report = crate::correct_rgba8(rgba, w, h, path);
    REPORT.with(|r| *r.borrow_mut() = report.to_json());
    0
}

/// The last report as UTF-8 JSON: pointer and length.
#[unsafe(no_mangle)]
pub extern "C" fn ac_report_ptr() -> *const u8 {
    REPORT.with(|r| r.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ac_report_len() -> usize {
    REPORT.with(|r| r.borrow().len())
}
