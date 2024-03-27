#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::VpnClient;
use openconnect_sys::{
    oc_form_opt_select, openconnect_set_option_value, OC_FORM_OPT_HIDDEN, OC_FORM_OPT_IGNORE,
    OC_FORM_OPT_PASSWORD, OC_FORM_OPT_SELECT, OC_FORM_OPT_TEXT, OC_FORM_OPT_TOKEN,
    OC_FORM_RESULT_CANCELLED, OC_FORM_RESULT_OK,
};
use std::{
    ffi::{CStr, CString},
    ptr,
};

pub struct FormField {
    pub form_id: String,
    pub opt_id: String,
    pub value: Option<String>,
}

#[repr(C)]
pub struct FormManager {
    last_form_empty: i32,
    saved_form_fields: Vec<FormField>, // TODO: currently not in use
}

// TODO: optimize this
impl FormManager {
    pub fn new() -> Self {
        Self {
            last_form_empty: -1,
            saved_form_fields: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.last_form_empty = -1;
        self.saved_form_fields.clear();
    }

    unsafe fn saved_form_field(
        &self,
        form_id: Option<&str>,
        opt_id: Option<&str>,
    ) -> Option<String> {
        let found = self
            .saved_form_fields
            .iter()
            .find(|ff| Some(ff.form_id.as_str()) == form_id && Some(ff.opt_id.as_str()) == opt_id);

        found.and_then(|ff| ff.value.to_owned())
    }

    // TODO: better impl rather than a C style
    unsafe fn match_choice_label(&self, select_opt: *mut oc_form_opt_select, label: &str) -> i32 {
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
            // TODO: review this
            let mut this = (*client)
                .form_manager
                .try_write()
                .expect("try_write form_context failed");

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

                        let auth_id = CStr::from_ptr((*form).auth_id).to_str().ok();
                        let opt_id = CStr::from_ptr((*select_opt).form.name).to_str().ok();
                        let opt_response = this.saved_form_field(auth_id, opt_id);

                        if opt_response.is_some()
                            && this.match_choice_label(select_opt, &opt_response.unwrap()) == 0
                        {
                            continue 'loop_opt;
                        }
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
                        let auth_id = CStr::from_ptr((*form).auth_id).to_str().ok();
                        let opt_id = CStr::from_ptr((*opt).name).to_str().ok();
                        let value = this.saved_form_field(auth_id, opt_id);
                        if value.is_some() {
                            let value = CString::new(value.unwrap()).unwrap();
                            openconnect_set_option_value(opt, value.as_ptr());
                        } else {
                            // TODO: implement prompt;
                        }
                    }
                    _ => {
                        continue 'loop_opt;
                    }
                }

                opt = (*opt).next;
            }

            // TODO: optimize this stupid empty check
            if empty != 0 {
                this.last_form_empty = 0;
            } else if {
                this.last_form_empty += 1;
                this.last_form_empty
            } >= 3
            {
                println!(
                    "{} consecutive empty forms, aborting loop",
                    this.last_form_empty
                );
                println!();
                return OC_FORM_RESULT_CANCELLED as i32;
            }
        }
        println!("Submitting form");
        println!();
        OC_FORM_RESULT_OK as i32
    }
}

impl Default for FormManager {
    fn default() -> Self {
        Self::new()
    }
}
