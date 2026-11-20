#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* Minimal C structs matching openconnect-sys/src/bindings_x86_64_linux_gnu.rs */
struct oc_ip_info {
    const char *addr;
    const char *netmask;
    const char *addr6;
    const char *netmask6;
    const char *dns[3];
    const char *nbns[3];
    const char *domain;
    const char *proxy_pac;
    int mtu;
    void *split_dns;
    void *split_includes;
    void *split_excludes;
    const char *gateway_addr;
};

struct oc_vpn_proto {
    const char *name;
    const char *pretty_name;
    const char *description;
    unsigned int flags;
};

struct oc_form_opt {
    struct oc_form_opt *next;
    int type;
    char *name;
    char *label;
    char *_value;
    unsigned int flags;
    void *reserved;
};

struct oc_choice {
    char *name;
    char *label;
    char *auth_type;
    char *override_name;
    char *override_label;
};

struct oc_form_opt_select {
    struct oc_form_opt form;
    int nr_choices;
    struct oc_choice **choices;
};

struct mock_vpninfo {
    struct oc_ip_info ip_info;
    const char *cookie;
    const char *hostname;
    const char *peer_cert_hash;
};

/* Tracks live allocations made by mock_get_supported_protocols */
static int g_proto_alloc_count = 0;

struct mock_vpninfo *mock_vpninfo_new(void) {
    struct mock_vpninfo *v = (struct mock_vpninfo *)calloc(1, sizeof(struct mock_vpninfo));
    v->ip_info.addr = "10.8.0.2";
    v->ip_info.netmask = "255.255.255.0";
    v->ip_info.mtu = 1400;
    v->ip_info.gateway_addr = "198.51.100.1";
    /* Before login/parse_url, libopenconnect leaves these pointers NULL */
    v->cookie = NULL;
    v->hostname = NULL;
    v->peer_cert_hash = NULL;
    return v;
}

void mock_vpninfo_free(struct mock_vpninfo *v) {
    free(v);
}

/* Matches libopenconnect library.c: openconnect_get_ip_info */
int mock_openconnect_get_ip_info(
    struct mock_vpninfo *vpninfo,
    const struct oc_ip_info **info,
    const void **cstp_options,
    const void **dtls_options
) {
    (void)cstp_options;
    (void)dtls_options;
    if (info) {
        *info = &vpninfo->ip_info;
    }
    return 0;
}

const char *mock_openconnect_get_cookie(struct mock_vpninfo *vpninfo) {
    return vpninfo->cookie;
}

const char *mock_openconnect_get_hostname(struct mock_vpninfo *vpninfo) {
    return vpninfo->hostname;
}

const char *mock_openconnect_get_peer_cert_hash(struct mock_vpninfo *vpninfo) {
    return vpninfo->peer_cert_hash;
}

/* Matches libopenconnect library.c: allocates array requiring openconnect_free_supported_protocols */
int mock_openconnect_get_supported_protocols(struct oc_vpn_proto **protos) {
    struct oc_vpn_proto *arr = (struct oc_vpn_proto *)calloc(2, sizeof(struct oc_vpn_proto));
    arr[0].name = "anyconnect";
    arr[0].pretty_name = "Cisco AnyConnect";
    arr[0].description = "Compatible with Cisco AnyConnect SSL VPN";
    arr[0].flags = 0;
    /* sentinel entry with NULL name */
    arr[1].name = NULL;
    *protos = arr;
    g_proto_alloc_count++;
    return 1;
}

void mock_openconnect_free_supported_protocols(struct oc_vpn_proto *protos) {
    if (protos) {
        free(protos);
        g_proto_alloc_count--;
    }
}

int mock_get_unfreed_proto_count(void) {
    return g_proto_alloc_count;
}

/* Matches libopenconnect free_auth_form: frees BOTH opt->_value AND choice->name/label */
struct oc_form_opt_select *mock_create_select_opt(const char *choice_name, const char *choice_label) {
    struct oc_form_opt_select *sel = (struct oc_form_opt_select *)calloc(1, sizeof(struct oc_form_opt_select));
    struct oc_choice *ch = (struct oc_choice *)calloc(1, sizeof(struct oc_choice));
    ch->name = strdup(choice_name);
    ch->label = strdup(choice_label);

    sel->form.type = 3; /* OC_FORM_OPT_SELECT */
    sel->form.name = strdup("group_list");
    sel->form._value = NULL;
    sel->nr_choices = 1;
    sel->choices = (struct oc_choice **)calloc(1, sizeof(struct oc_choice *));
    sel->choices[0] = ch;
    return sel;
}

void mock_free_select_opt(struct oc_form_opt_select *sel) {
    if (!sel) return;
    /* libopenconnect frees opt->_value assuming it was allocated via openconnect_set_option_value (strdup) */
    free(sel->form._value);
    free(sel->form.name);
    for (int i = 0; i < sel->nr_choices; i++) {
        free(sel->choices[i]->name);
        free(sel->choices[i]->label);
        free(sel->choices[i]);
    }
    free(sel->choices);
    free(sel);
}
