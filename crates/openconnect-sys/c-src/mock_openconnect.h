#ifndef MOCK_OPENCONNECT_H
#define MOCK_OPENCONNECT_H

struct openconnect_info;

void mock_oc_reset_counters(void);
int mock_oc_live_vpninfo_count(void);
int mock_oc_live_proto_alloc_count(void);
int mock_oc_set_return_nulls(struct openconnect_info *vpninfo, int enabled);
void mock_oc_trigger_progress_empty(struct openconnect_info *vpninfo);
void mock_oc_trigger_progress_unknown_level(struct openconnect_info *vpninfo);
int mock_oc_set_peer_cert_hash(struct openconnect_info *vpninfo, const char *hash);
int mock_oc_trigger_validate_peer_cert(struct openconnect_info *vpninfo, const char *reason);
void mock_oc_trigger_stats(struct openconnect_info *vpninfo, int include_stats);
void mock_oc_trigger_setup_tun(struct openconnect_info *vpninfo);
int mock_oc_trigger_auth_form_ignored_opt(struct openconnect_info *vpninfo);
int mock_oc_trigger_auth_form_select(
    struct openconnect_info *vpninfo,
    const char *choice_name,
    const char *choice_label,
    const char *opt_name
);
int mock_oc_trigger_auth_form_text_and_password(
    struct openconnect_info *vpninfo,
    const char *text_field_name
);
int mock_oc_trigger_auth_form_null_cases(struct openconnect_info *vpninfo);

#endif
