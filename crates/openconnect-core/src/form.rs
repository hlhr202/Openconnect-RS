#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::VpnClient;
use openconnect_sys::{
    oc_form_opt_select, openconnect_info, openconnect_set_option_value, OC_FORM_OPT_HIDDEN,
    OC_FORM_OPT_IGNORE, OC_FORM_OPT_PASSWORD, OC_FORM_OPT_SELECT, OC_FORM_OPT_TEXT,
    OC_FORM_OPT_TOKEN, OC_FORM_RESULT_CANCELLED, OC_FORM_RESULT_OK,
};
use std::{ffi::CString, ptr};

pub struct FormField {
    pub next: *mut FormField,
    pub form_id: *mut i8,
    pub opt_id: *mut i8,
    pub value: Option<String>,
}

pub static mut FORM_FIELDS: *mut FormField = std::ptr::null_mut();
static mut LAST_FORM_EMPTY: i32 = -1;

#[repr(C)]
pub struct Form {
    _unused: [u8; 0],
}

// TODO: optimize this
impl Form {
    unsafe fn saved_form_field(
        _vpninfo: *mut openconnect_info,
        form_id: *mut i8,
        opt_id: *mut i8,
        found: *mut i32,
    ) -> Option<String> {
        println!("saved_form_field");
        let mut ff = FORM_FIELDS;

        while !ff.is_null() {
            if (*ff).form_id == form_id && (*ff).opt_id == opt_id {
                if !found.is_null() {
                    *found = 1;
                }
                return (*ff).value.clone();
            }

            ff = (*ff).next;
        }

        if !found.is_null() {
            *found = 0;
        }

        None
    }

    unsafe fn match_choice_label(
        _vpninfo: *mut openconnect_info,
        select_opt: *mut oc_form_opt_select,
        label: &str,
    ) -> i32 {
        let mut match_ = ptr::null_mut::<i8>();

        let input_len = label.len();
        let mut partial_matches = 0;

        if input_len < 1 {
            return -1;
        }

        for i in 0..(*select_opt).nr_choices {
            let choice = *(*select_opt).choices.offset(i as isize);
            let choice_label = std::ffi::CStr::from_ptr((*choice).label).to_str().unwrap();
            if label[..input_len].eq_ignore_ascii_case(&choice_label[..input_len]) {
                if choice_label.len() == input_len {
                    (*select_opt).form._value = (*choice).name;
                    return 0;
                }
                match_ = (*choice).label;
                partial_matches += 1;
            }
        }

        if partial_matches == 1 {
            (*select_opt).form._value = match_;
            return 0;
        }
        if partial_matches > 1 {
            println!("Auth choice '{}' is ambiguous", label);
            -libc::EINVAL
        } else {
            println!("Auth choice '{}' not found", label);
            -libc::EINVAL
        }
    }

    // TODO: forward rust string to C
    #[no_mangle]
    pub extern "C" fn process_auth_form_cb(
        privdata: *mut ::std::os::raw::c_void,
        form: *mut openconnect_sys::oc_auth_form,
    ) -> ::std::os::raw::c_int {
        println!("process_auth_form_cb");
        let client = VpnClient::from_c_void(privdata);
        unsafe {
            let vpninfo = (*client).vpninfo;
            let mut opt = (*form).opts;
            let mut empty = 1;

            if (*form).auth_id.is_null() {
                return -libc::EINVAL;
            }

            if !(*form).error.is_null() {
                let error: String = std::ffi::CStr::from_ptr((*form).error)
                    .to_string_lossy()
                    .into();
                println!("Authentication failed: {}", error);
            }

            if !(*form).authgroup_opt.is_null() {
                // TODO: implement authgroup
                println!("authgroup_opt");
            }

            'loop_opt: while !opt.is_null() {
                if ((*opt).flags & OC_FORM_OPT_IGNORE) != 0 {
                    continue 'loop_opt;
                }

                match (*opt).type_ as u32 {
                    OC_FORM_OPT_SELECT => {
                        println!("OC_FORM_OPT_SELECT");
                        let select_opt = opt.cast::<oc_form_opt_select>();

                        if select_opt == (*form).authgroup_opt {
                            continue 'loop_opt;
                        }

                        let opt_response = Form::saved_form_field(
                            vpninfo,
                            (*form).auth_id,
                            (*select_opt).form.name,
                            ptr::null_mut(),
                        );

                        if opt_response.is_some()
                            && Form::match_choice_label(vpninfo, select_opt, &opt_response.unwrap())
                                == 0
                        {
                            // free(opt_response);
                            continue 'loop_opt;
                        }
                        // free(opt_response);
                        // TODO: if (prompt_opt_select(vpninfo, form, select_opt) < 0)
                        //     goto error;
                        empty = 0;
                    }
                    OC_FORM_OPT_TEXT => {
                        let opt_name = std::ffi::CStr::from_ptr((*opt).name).to_str().unwrap();
                        let value = (*client).handle_text_input(opt_name);
                        if let Some(value) = value {
                            let value = CString::new(value).unwrap();
                            openconnect_set_option_value(opt, value.as_ptr());

                            // if (*client).form_attempt == 0
                            //     && (opt_name == "user" || opt_name == "uname" || opt_name == "username")
                            // {
                            //     openconnect_set_option_value(opt, user.as_c_str().as_ptr());
                            // } else {
                            //     let value = Form::saved_form_field(
                            //         vpninfo,
                            //         (*form).auth_id,
                            //         (*opt).name,
                            //         ptr::null_mut(),
                            //     );
                            //     if value.is_some() {
                            //         let value = CString::new(value.unwrap()).unwrap();
                            //         openconnect_set_option_value(opt, value.as_ptr());
                            //     }
                            // }

                            if (*opt)._value.is_null() {
                                println!("No value for {}", opt_name);
                                // goto error;
                            }
                            empty = 0;
                        }
                    }
                    OC_FORM_OPT_PASSWORD => {
                        let value = (*client).handle_password_input();
                        if let Some(value) = value {
                            let value = CString::new(value).unwrap();
                            openconnect_set_option_value(opt, value.as_ptr());
                            empty = 0;
                        }
                    }
                    OC_FORM_OPT_TOKEN => {
                        println!("OC_FORM_OPT_TOKEN");
                        // Nothing to do here
                        empty = 0;
                    }
                    OC_FORM_OPT_HIDDEN => {
                        println!("OC_FORM_OPT_HIDDEN");
                        let found = ptr::null_mut::<i32>();
                        let value =
                            Form::saved_form_field(vpninfo, (*form).auth_id, (*opt).name, found);
                        if value.is_some() {
                            let value = CString::new(value.unwrap()).unwrap();
                            openconnect_set_option_value(opt, value.as_ptr());
                        } else if !found.is_null() {
                            // TODO: implement prompt;
                        }
                    }
                    _ => {
                        continue 'loop_opt;
                    }
                }

                opt = (*opt).next;
            }

            if empty != 0 {
                LAST_FORM_EMPTY = 0;
            } else if {
                LAST_FORM_EMPTY += 1;
                LAST_FORM_EMPTY
            } >= 3
            {
                println!("{} consecutive empty forms, aborting loop", LAST_FORM_EMPTY);
                println!();
                return OC_FORM_RESULT_CANCELLED as i32;
            }
        }
        println!("Submitting form");
        println!();
        OC_FORM_RESULT_OK as i32
    }
}
