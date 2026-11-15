//! The C ABI as the browser uses it.
mod common;
use autocolor_engine::abi::*;
use common::*;

#[test]
fn a_photo_round_trips_through_the_c_abi() {
    let img = fixture("people");
    let len = img.rgba.len();
    let dark: Vec<u8> = img.rgba.iter().enumerate().map(|(i, &v)| if i % 4 == 3 { v } else { v / 3 }).collect();
    let ptr = ac_alloc(len);
    assert!(!ptr.is_null());
    unsafe {
        std::slice::from_raw_parts_mut(ptr, len).copy_from_slice(&dark);
        assert_eq!(ac_correct(ptr, img.w as u32, img.h as u32, 1), 0);
        let out = std::slice::from_raw_parts(ptr, len).to_vec();
        assert!(mean_l(&out) > mean_l(&dark) + 5.0);
        let report = std::str::from_utf8(std::slice::from_raw_parts(ac_report_ptr(), ac_report_len())).unwrap().to_string();
        assert!(report.starts_with("{\"path\":\"banded\",\"choice\":{\"exposure\":"), "{report}");
        assert!(report.ends_with("}}"), "{report}");
        assert_eq!(ac_correct(ptr, img.w as u32, img.h as u32, 7), -2);
        assert_eq!(ac_correct(std::ptr::null_mut(), 1, 1, 0), -1);
        ac_free(ptr, len);
    }
    assert!(ac_full_bytes(4000, 3000) > 1.0e9);
}
