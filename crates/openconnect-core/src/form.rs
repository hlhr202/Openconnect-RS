use crate::{errno::EINVAL, OpenconnectCtx};
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
static mut LAST_FORM_EMPTY: i32 = 1;

#[repr(C)]
pub struct Form {
    _unused: [u8; 0],
}

impl Form {
    unsafe fn saved_form_field(
        _vpninfo: *mut openconnect_info,
        form_id: *mut i8,
        opt_id: *mut i8,
        found: *mut i32,
    ) -> Option<String> {
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
            -EINVAL
        } else {
            println!("Auth choice '{}' not found", label);
            -EINVAL
        }
    }

    // TODO: forward rust string to C
    #[no_mangle]
    pub extern "C" fn process_auth_form_cb(
        privdata: *mut ::std::os::raw::c_void,
        form: *mut openconnect_sys::oc_auth_form,
    ) -> ::std::os::raw::c_int {
        println!("process_auth_form_cb");
        let ctx = OpenconnectCtx::from_c_void(privdata);
        unsafe {
            let user = (*ctx).user.clone();
            let password = (*ctx).password.clone();

            let vpninfo = (*ctx).vpninfo;
            let mut opt = (*form).opts;
            let mut empty = 1;

            if (*form).auth_id.is_null() {
                return -1;
            }

            if !(*form).error.is_null() {
                let error: String = std::ffi::CStr::from_ptr((*form).error)
                    .to_string_lossy()
                    .into();
                println!("Authentication failed: {}", error);
            }

            if !(*form).authgroup_opt.is_null() {
                // TODO: authgroup
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
                        println!("OC_FORM_OPT_TEXT: {}", opt_name);

                        if opt_name == "user" || opt_name == "uname" || opt_name == "username" {
                            openconnect_set_option_value(opt, user.as_ptr());
                        } else {
                            let value = Form::saved_form_field(
                                vpninfo,
                                (*form).auth_id,
                                (*opt).name,
                                ptr::null_mut(),
                            );
                            if value.is_some() {
                                let value = CString::new(value.unwrap()).unwrap();
                                openconnect_set_option_value(opt, value.as_ptr());
                            }

                            #[allow(unused_labels)]
                            'prompt: {
                                // TODO: (*opt)._value = prompt_for_input(vpninfo, form, opt);
                            }
                        }

                        if (*opt)._value.is_null() {
                            println!("No value for {}", opt_name);
                            // goto error;
                        } else {
                            let name_fmt = std::ffi::CString::new("name: %s\n").unwrap();
                            libc::printf(name_fmt.as_ptr(), (*opt)._value);
                        }

                        empty = 0;
                    }
                    OC_FORM_OPT_PASSWORD => {
                        println!("OC_FORM_OPT_PASSWORD");
                        openconnect_set_option_value(opt, password.as_ptr());
                        let password_fmt = std::ffi::CString::new("password length: %d\n").unwrap();
                        let password_len = libc::strlen((*opt)._value);
                        libc::printf(password_fmt.as_ptr(), password_len);

                        empty = 0;
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
                            // TODO: goto prompt;
                        }
                    }
                    _ => {
                        continue 'loop_opt;
                    }
                }

                opt = (*opt).next;
            }

            if empty == 0 {
                LAST_FORM_EMPTY = 1;
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
