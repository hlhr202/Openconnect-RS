//! Fixture 06: Proves 3 critical bugs in `crates/openconnect-core/src/form.rs`:
//!   (a) Double-free in `match_choice_label` (assigning `(*choice).name` directly to `form._value`
//!       instead of calling `openconnect_set_option_value`, caught by ASan/glibc).
//!   (b) Infinite loop in `process_auth_form_cb` when `OC_FORM_OPT_IGNORE` (or unknown/authgroup)
//!       executes `continue 'loop_opt` before `opt = (*opt).next`.
//!   (c) Out-of-bounds slice panic `&choice_label[..input_len]` when `label.len() > choice_label.len()`.

use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::panic;

const OC_FORM_OPT_IGNORE: c_uint = 1;

#[repr(C)]
struct OcFormOpt {
    next: *mut OcFormOpt,
    type_: c_int,
    name: *mut c_char,
    label: *mut c_char,
    _value: *mut c_char,
    flags: c_uint,
    reserved: *mut c_void,
}

#[repr(C)]
struct OcChoice {
    name: *mut c_char,
    label: *mut c_char,
    auth_type: *mut c_char,
    override_name: *mut c_char,
    override_label: *mut c_char,
}

#[repr(C)]
struct OcFormOptSelect {
    form: OcFormOpt,
    nr_choices: c_int,
    choices: *mut *mut OcChoice,
}

extern "C" {
    fn mock_create_select_opt(
        choice_name: *const c_char,
        choice_label: *const c_char,
    ) -> *mut OcFormOptSelect;
    fn mock_free_select_opt(sel: *mut OcFormOptSelect);
}

/// Exact reproduction of `crates/openconnect-core/src/form.rs:51-72`
unsafe fn buggy_match_choice_label(select_opt: *mut OcFormOptSelect, label: &str) -> i32 {
    let input_len = label.len();
    for i in 0..(*select_opt).nr_choices {
        let choice = *(*select_opt).choices.offset(i as isize);
        let choice_label = CStr::from_ptr((*choice).label).to_str().unwrap();
        // BUG (c): panics when input_len > choice_label.len() or non-char-boundary!
        if label[..input_len].eq_ignore_ascii_case(&choice_label[..input_len]) {
            if choice_label.len() == input_len {
                // BUG (a): Direct pointer alias -> C's free_auth_form frees BOTH _value and choice.name!
                (*select_opt).form._value = (*choice).name;
                return 0;
            }
        }
    }
    -22
}

/// Exact reproduction of `crates/openconnect-core/src/form.rs:122-211` (with iteration watchdog)
unsafe fn buggy_form_loop_hits_infinite_loop(mut opt: *mut OcFormOpt) -> bool {
    let mut iterations = 0usize;
    'loop_opt: while !opt.is_null() {
        iterations += 1;
        if iterations > 10_000 {
            return true; // Infinite loop detected!
        }
        if ((*opt).flags & OC_FORM_OPT_IGNORE) != 0 {
            continue 'loop_opt; // <-- BUG (b): skips `opt = (*opt).next`!
        }
        opt = (*opt).next;
    }
    false
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "safe_checks".to_string());

    if mode == "--double-free" {
        // Trigger (a) under ASan/Valgrind to prove heap double-free
        unsafe {
            let sel = mock_create_select_opt(c"corp".as_ptr(), c"Corp".as_ptr());
            assert_eq!(buggy_match_choice_label(sel, "Corp"), 0);
            mock_free_select_opt(sel); // <-- DOUBLE FREE here!
        }
        return;
    }

    // Prove (b): infinite loop on OC_FORM_OPT_IGNORE
    unsafe {
        let mut ignored_opt = OcFormOpt {
            next: std::ptr::null_mut(),
            type_: 0,
            name: std::ptr::null_mut(),
            label: std::ptr::null_mut(),
            _value: std::ptr::null_mut(),
            flags: OC_FORM_OPT_IGNORE,
            reserved: std::ptr::null_mut(),
        };
        assert!(buggy_form_loop_hits_infinite_loop(&mut ignored_opt));
        println!("[06b] PROVEN (Infinite loop): `continue 'loop_opt` on OC_FORM_OPT_IGNORE never advances `opt = (*opt).next`!");
    }

    // Prove (c): slice OOB panic inside extern "C" callback when input_len > choice_label.len()
    unsafe {
        let sel = mock_create_select_opt(c"vpn".as_ptr(), c"AB".as_ptr());
        let res = panic::catch_unwind(|| buggy_match_choice_label(sel, "LONGER_THAN_AB"));
        assert!(res.is_err(), "Expected slice out-of-bounds panic");
        mock_free_select_opt(sel);
        println!("[06c] PROVEN (Slice OOB panic across FFI): `&choice_label[..input_len]` panicked when label > choice_label!");
    }
}
