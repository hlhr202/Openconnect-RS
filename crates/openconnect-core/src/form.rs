use crate::ffi::{CallbackContext, c_str_to_option_string, with_callback_context};
use openconnect_sys::{
    OC_FORM_OPT_HIDDEN, OC_FORM_OPT_IGNORE, OC_FORM_OPT_PASSWORD, OC_FORM_OPT_SELECT,
    OC_FORM_OPT_TEXT, OC_FORM_OPT_TOKEN, OC_FORM_RESULT_CANCELLED, OC_FORM_RESULT_ERR,
    OC_FORM_RESULT_OK, oc_auth_form, oc_form_opt_select, openconnect_set_option_value,
};
use std::ffi::{CString, c_char, c_int, c_void};

pub struct FormField {
    pub form_id: String,
    pub opt_id: String,
    pub value: Option<String>,
}

pub struct FormManager {
    last_form_empty: i32,
    saved_form_fields: Vec<FormField>,
}

impl FormManager {
    pub fn new() -> Self {
        Self {
            last_form_empty: 0,
            saved_form_fields: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.last_form_empty = 0;
        self.saved_form_fields.clear();
    }

    #[cfg(feature = "mock-ffi")]
    pub fn save_field(&mut self, form_id: String, opt_id: String, value: Option<String>) {
        self.saved_form_fields.push(FormField {
            form_id,
            opt_id,
            value,
        });
    }

    fn saved_form_field(&self, form_id: Option<&str>, opt_id: Option<&str>) -> Option<String> {
        let found = self
            .saved_form_fields
            .iter()
            .find(|ff| Some(ff.form_id.as_str()) == form_id && Some(ff.opt_id.as_str()) == opt_id);

        found.and_then(|ff| ff.value.clone())
    }

    /// Match a saved choice label against a C `oc_form_opt_select` and set the option value
    /// using `openconnect_set_option_value` (which copies the string safely on the C heap).
    ///
    /// # Safety
    /// `select_opt` must be a valid pointer to an `oc_form_opt_select` provided by `libopenconnect`.
    unsafe fn match_choice_label(&self, select_opt: *mut oc_form_opt_select, label: &str) -> i32 {
        if select_opt.is_null() || label.is_empty() {
            return -libc::EINVAL;
        }

        // SAFETY: `select_opt` is verified non-null and valid per caller contract.
        let select_ref = unsafe { &*select_opt };
        let nr_choices = select_ref.nr_choices;
        let choices_ptr = select_ref.choices;
        if nr_choices <= 0 || choices_ptr.is_null() {
            return -libc::EINVAL;
        }
        let Ok(nr_choices) = usize::try_from(nr_choices) else {
            return -libc::EINVAL;
        };

        // SAFETY: `libopenconnect` guarantees `choices_ptr` points to `nr_choices` choice pointers.
        let choices = unsafe { std::slice::from_raw_parts(choices_ptr, nr_choices) };

        let input_len = label.len();
        let mut partial_match_name: *const c_char = std::ptr::null();
        let mut partial_matches = 0usize;

        for &choice_ptr in choices {
            if choice_ptr.is_null() {
                continue;
            }
            // SAFETY: `choice_ptr` is verified non-null and valid within `select_opt.choices`.
            let choice = unsafe { &*choice_ptr };
            let choice_label_ptr = choice.label;
            let choice_name_ptr = choice.name;

            if choice_name_ptr.is_null() {
                continue;
            }

            // SAFETY: `choice_label_ptr` is either NULL or a valid C string owned by `choice_ptr`.
            let Some(choice_label) = (unsafe { c_str_to_option_string(choice_label_ptr) }) else {
                continue;
            };

            if choice_label.len() >= input_len
                && choice_label.is_char_boundary(input_len)
                && choice_label[..input_len].eq_ignore_ascii_case(label)
            {
                if choice_label.len() == input_len {
                    // `oc_form_opt` is the first field of `#[repr(C)] oc_form_opt_select`.
                    let opt_ptr = select_opt.cast::<openconnect_sys::oc_form_opt>();
                    // SAFETY: `openconnect_set_option_value` duplicates `choice_name_ptr` on the C heap,
                    // avoiding any double-free when `libopenconnect` frees both `_value` and `choice->name`.
                    return unsafe { openconnect_set_option_value(opt_ptr, choice_name_ptr) };
                }
                partial_match_name = choice_name_ptr;
                partial_matches += 1;
            }
        }

        if partial_matches == 1 && !partial_match_name.is_null() {
            let opt_ptr = select_opt.cast::<openconnect_sys::oc_form_opt>();
            // SAFETY: `openconnect_set_option_value` duplicates `partial_match_name` on the C heap.
            return unsafe { openconnect_set_option_value(opt_ptr, partial_match_name) };
        }

        if partial_matches > 1 {
            tracing::warn!("Auth choice '{}' is ambiguous", label);
        } else {
            tracing::warn!("Auth choice '{}' not found", label);
        }
        -libc::EINVAL
    }

    pub(crate) extern "C" fn process_auth_form_cb(
        privdata: *mut c_void,
        form: *mut oc_auth_form,
    ) -> c_int {
        tracing::debug!("Calling process_auth_form_cb");

        let body = |ctx: &CallbackContext| -> c_int {
            if form.is_null() {
                tracing::error!("Received NULL oc_auth_form in process_auth_form_cb");
                return -libc::EINVAL;
            }

            let Ok(mut this) = ctx.state.form_manager.try_lock() else {
                tracing::error!("form_manager lock contention in process_auth_form_cb");
                return OC_FORM_RESULT_ERR;
            };

            // SAFETY: `form` was verified non-null and is provided by `libopenconnect`.
            let form_ref = unsafe { &*form };
            let auth_id_ptr = form_ref.auth_id;
            let error_ptr = form_ref.error;
            let authgroup_opt = form_ref.authgroup_opt;
            let mut opt = form_ref.opts;

            // SAFETY: `auth_id_ptr` is checked for null inside `c_str_to_option_string`.
            let Some(auth_id) = (unsafe { c_str_to_option_string(auth_id_ptr) }) else {
                return -libc::EINVAL;
            };

            // SAFETY: `error_ptr` is checked for null inside `c_str_to_option_string`.
            if let Some(error) = unsafe { c_str_to_option_string(error_ptr) } {
                tracing::warn!("Authentication form error: {}", error);
            }

            if !authgroup_opt.is_null() {
                tracing::debug!("authgroup_opt present");
            }

            let mut empty = true;

            while !opt.is_null() {
                // Advance `next` upfront so every `continue` moves forward (no infinite loops!).
                // SAFETY: `opt` is verified non-null in the loop condition.
                let opt_ref = unsafe { &*opt };
                let next_opt = opt_ref.next;
                let flags = opt_ref.flags;
                let opt_type = opt_ref.type_;
                let opt_name_ptr = opt_ref.name;
                let current_opt = opt;
                opt = next_opt;

                if (flags & OC_FORM_OPT_IGNORE) != 0 {
                    continue;
                }

                match opt_type as u32 {
                    OC_FORM_OPT_SELECT => {
                        let select_opt = current_opt.cast::<oc_form_opt_select>();
                        if select_opt == authgroup_opt {
                            continue;
                        }

                        // SAFETY: `select_opt` has `oc_form_opt` as its first field (`#[repr(C)]`).
                        let select_name_ptr = unsafe { (*select_opt).form.name };
                        // SAFETY: `select_name_ptr` is checked for null inside helper.
                        let opt_id = unsafe { c_str_to_option_string(select_name_ptr) };
                        let opt_response =
                            this.saved_form_field(Some(auth_id.as_str()), opt_id.as_deref());

                        if let Some(response) = opt_response {
                            // SAFETY: `select_opt` is a valid `OC_FORM_OPT_SELECT` pointer.
                            if unsafe { this.match_choice_label(select_opt, &response) } == 0 {
                                empty = false;
                                continue;
                            }
                        }
                        empty = false;
                    }
                    OC_FORM_OPT_TEXT => {
                        // SAFETY: `opt_name_ptr` is checked for null inside helper.
                        let Some(opt_name) = (unsafe { c_str_to_option_string(opt_name_ptr) })
                        else {
                            tracing::warn!("Skipping OC_FORM_OPT_TEXT with NULL name");
                            continue;
                        };

                        if let Some(value) = ctx.state.handle_text_input(&opt_name) {
                            let Ok(value_cstr) = CString::new(value) else {
                                tracing::error!(
                                    "Text input for '{}' contains interior NUL",
                                    opt_name
                                );
                                return OC_FORM_RESULT_ERR;
                            };
                            // SAFETY: `current_opt` and `value_cstr` are valid for this call.
                            if unsafe {
                                openconnect_set_option_value(current_opt, value_cstr.as_ptr())
                            } != 0
                            {
                                return OC_FORM_RESULT_ERR;
                            }
                            empty = false;
                        }
                    }
                    OC_FORM_OPT_PASSWORD => {
                        if let Some(value) = ctx.state.handle_password_input() {
                            let Ok(value_cstr) = CString::new(value) else {
                                tracing::error!("Password input contains interior NUL");
                                return OC_FORM_RESULT_ERR;
                            };
                            // SAFETY: `current_opt` and `value_cstr` are valid for this call.
                            if unsafe {
                                openconnect_set_option_value(current_opt, value_cstr.as_ptr())
                            } != 0
                            {
                                return OC_FORM_RESULT_ERR;
                            }
                            empty = false;
                        }
                    }
                    OC_FORM_OPT_TOKEN => {
                        tracing::debug!("OC_FORM_OPT_TOKEN encountered");
                        empty = false;
                    }
                    OC_FORM_OPT_HIDDEN => {
                        // SAFETY: `opt_name_ptr` is checked for null inside helper.
                        let opt_id = unsafe { c_str_to_option_string(opt_name_ptr) };
                        if let Some(value) =
                            this.saved_form_field(Some(auth_id.as_str()), opt_id.as_deref())
                        {
                            let Ok(value_cstr) = CString::new(value) else {
                                return OC_FORM_RESULT_ERR;
                            };
                            // SAFETY: `current_opt` and `value_cstr` are valid for this call.
                            if unsafe {
                                openconnect_set_option_value(current_opt, value_cstr.as_ptr())
                            } != 0
                            {
                                return OC_FORM_RESULT_ERR;
                            }
                        }
                    }
                    _ => continue,
                }
            }

            if !empty {
                this.last_form_empty = 0;
            } else {
                this.last_form_empty += 1;
                if this.last_form_empty >= 3 {
                    tracing::debug!(
                        "{} consecutive empty forms, aborting loop",
                        this.last_form_empty
                    );
                    return OC_FORM_RESULT_CANCELLED as c_int;
                }
            }

            tracing::debug!("Successfully processed auth form");
            OC_FORM_RESULT_OK as c_int
        };

        // SAFETY: Invoked synchronously by `libopenconnect` while `VpnInfoHandle` is alive.
        unsafe { with_callback_context(privdata, OC_FORM_RESULT_ERR, body) }
    }
}

impl Default for FormManager {
    fn default() -> Self {
        Self::new()
    }
}
