#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef _WIN32
#include <unistd.h>
#endif

#include "helper.h"
#include "mock_openconnect.h"

/* Struct layouts strictly matching openconnect.h / bindings_x86_64_linux_gnu.rs */
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

struct oc_auth_form {
    char *banner;
    char *message;
    char *error;
    char *auth_id;
    char *method;
    char *action;
    struct oc_form_opt *opts;
    struct oc_form_opt_select *authgroup_opt;
    int authgroup_selection;
};

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

struct oc_vpn_option {
    char *option;
    char *value;
    struct oc_vpn_option *next;
};

struct oc_stats {
    uint64_t tx_pkts;
    uint64_t tx_bytes;
    uint64_t rx_pkts;
    uint64_t rx_bytes;
};

struct oc_vpn_proto {
    const char *name;
    const char *pretty_name;
    const char *description;
    unsigned int flags;
};

typedef int (*openconnect_validate_peer_cert_vfn)(void *privdata, const char *reason);
typedef int (*openconnect_write_new_config_vfn)(void *privdata, const char *buf, int buflen);
typedef int (*openconnect_process_auth_form_vfn)(void *privdata, struct oc_auth_form *form);
typedef void (*openconnect_progress_vfn)(void *privdata, int level, const char *fmt, ...);
typedef void (*openconnect_stats_vfn)(void *privdata, const struct oc_stats *stats);
typedef void (*openconnect_setup_tun_vfn)(void *privdata);

struct openconnect_info {
    char *useragent;
    char *protocol;
    char *reported_os;
    char *cookie;
    char *hostname;
    char *http_proxy;
    char *peer_cert_hash;
    char *dtls_cipher;
    int port;
    int loglevel;
    int dtls_disabled;
    int return_nulls;
    int cmd_pipe_read_fd;
    int cmd_pipe_write_fd;
    struct oc_ip_info ip_info;
    openconnect_validate_peer_cert_vfn validate_peer_cert;
    openconnect_write_new_config_vfn write_new_config;
    openconnect_process_auth_form_vfn process_auth_form;
    openconnect_progress_vfn progress;
    openconnect_stats_vfn stats_handler;
    openconnect_setup_tun_vfn setup_tun;
    void *cbdata;
};

static atomic_int g_live_vpninfo_count = 0;
static atomic_int g_live_proto_alloc_count = 0;

static char *dup_or_null(const char *s)
{
    return s ? strdup(s) : NULL;
}

void mock_oc_reset_counters(void)
{
    atomic_store(&g_live_vpninfo_count, 0);
    atomic_store(&g_live_proto_alloc_count, 0);
}

int mock_oc_live_vpninfo_count(void)
{
    return atomic_load(&g_live_vpninfo_count);
}

int mock_oc_live_proto_alloc_count(void)
{
    return atomic_load(&g_live_proto_alloc_count);
}

int mock_oc_set_return_nulls(struct openconnect_info *vpninfo, int enabled)
{
    if (!vpninfo) {
        return -1;
    }
    vpninfo->return_nulls = enabled;
    return 0;
}

int openconnect_init_ssl(void)
{
    return 0;
}

const char *openconnect_get_version(void)
{
    return "v9.12-mock";
}

struct openconnect_info *openconnect_vpninfo_new(
    const char *useragent,
    openconnect_validate_peer_cert_vfn validate_peer_cert,
    openconnect_write_new_config_vfn write_new_config,
    openconnect_process_auth_form_vfn process_auth_form,
    openconnect_progress_vfn progress,
    void *privdata
)
{
    struct openconnect_info *v = (struct openconnect_info *)calloc(1, sizeof(*v));
    if (!v) {
        return NULL;
    }
    v->useragent = dup_or_null(useragent);
    v->validate_peer_cert = validate_peer_cert;
    v->write_new_config = write_new_config;
    v->process_auth_form = process_auth_form;
    v->progress = progress;
    v->cbdata = privdata;
    v->port = 443;
    v->cmd_pipe_read_fd = -1;
    v->cmd_pipe_write_fd = -1;
    v->peer_cert_hash = strdup("pin-sha256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=");
    v->dtls_cipher = strdup("TLS_AES_256_GCM_SHA384");
    v->ip_info.addr = "10.8.0.2";
    v->ip_info.netmask = "255.255.255.0";
    v->ip_info.dns[0] = "1.1.1.1";
    v->ip_info.dns[1] = "8.8.8.8";
    v->ip_info.domain = "vpn.example.test";
    v->ip_info.mtu = 1400;
    v->ip_info.gateway_addr = "198.51.100.1";
    atomic_fetch_add(&g_live_vpninfo_count, 1);
    return v;
}

void openconnect_vpninfo_free(struct openconnect_info *vpninfo)
{
    if (!vpninfo) {
        return;
    }
#ifndef _WIN32
    if (vpninfo->cmd_pipe_read_fd >= 0) {
        close(vpninfo->cmd_pipe_read_fd);
    }
    if (vpninfo->cmd_pipe_write_fd >= 0) {
        close(vpninfo->cmd_pipe_write_fd);
    }
#endif
    free(vpninfo->useragent);
    free(vpninfo->protocol);
    free(vpninfo->reported_os);
    free(vpninfo->cookie);
    free(vpninfo->hostname);
    free(vpninfo->http_proxy);
    free(vpninfo->peer_cert_hash);
    free(vpninfo->dtls_cipher);
    free(vpninfo);
    atomic_fetch_sub(&g_live_vpninfo_count, 1);
}

void openconnect_set_loglevel(struct openconnect_info *vpninfo, int val)
{
    if (vpninfo) {
        vpninfo->loglevel = val;
    }
}

int openconnect_set_protocol(struct openconnect_info *vpninfo, const char *protocol)
{
    if (!vpninfo || !protocol) {
        return -22;
    }
    free(vpninfo->protocol);
    vpninfo->protocol = strdup(protocol);
    return 0;
}

void openconnect_set_stats_handler(struct openconnect_info *vpninfo, openconnect_stats_vfn stats_handler)
{
    if (vpninfo) {
        vpninfo->stats_handler = stats_handler;
    }
}

void openconnect_set_setup_tun_handler(struct openconnect_info *vpninfo, openconnect_setup_tun_vfn setup_tun)
{
    if (vpninfo) {
        vpninfo->setup_tun = setup_tun;
    }
}

int openconnect_setup_tun_device(
    struct openconnect_info *vpninfo,
    const char *vpnc_script,
    const char *ifname
)
{
    (void)vpnc_script;
    (void)ifname;
    return vpninfo ? 0 : -22;
}

int openconnect_set_reported_os(struct openconnect_info *vpninfo, const char *os)
{
    if (!vpninfo || !os) {
        return -22;
    }
    free(vpninfo->reported_os);
    vpninfo->reported_os = strdup(os);
    return 0;
}

int openconnect_obtain_cookie(struct openconnect_info *vpninfo)
{
    if (!vpninfo) {
        return -22;
    }
    free(vpninfo->cookie);
    vpninfo->cookie = strdup("mock-webvpn-cookie=12345");
    return 0;
}

int openconnect_set_cookie(struct openconnect_info *vpninfo, const char *cookie)
{
    if (!vpninfo || !cookie) {
        return -22;
    }
    free(vpninfo->cookie);
    vpninfo->cookie = strdup(cookie);
    return 0;
}

void openconnect_clear_cookie(struct openconnect_info *vpninfo)
{
    if (vpninfo) {
        free(vpninfo->cookie);
        vpninfo->cookie = NULL;
    }
}

const char *openconnect_get_cookie(struct openconnect_info *vpninfo)
{
    if (!vpninfo || vpninfo->return_nulls) {
        return NULL;
    }
    return vpninfo->cookie;
}

#ifdef _WIN32
uintptr_t openconnect_setup_cmd_pipe(struct openconnect_info *vpninfo)
{
    if (!vpninfo) {
        return (uintptr_t)(~(uintptr_t)0); /* INVALID_SOCKET */
    }
    return 0;
}
#else
int openconnect_setup_cmd_pipe(struct openconnect_info *vpninfo)
{
    if (!vpninfo) {
        return -22;
    }
    if (vpninfo->cmd_pipe_read_fd >= 0) {
        close(vpninfo->cmd_pipe_read_fd);
        vpninfo->cmd_pipe_read_fd = -1;
    }
    if (vpninfo->cmd_pipe_write_fd >= 0) {
        close(vpninfo->cmd_pipe_write_fd);
        vpninfo->cmd_pipe_write_fd = -1;
    }
    int fds[2];
    if (pipe(fds) != 0) {
        return -1;
    }
    vpninfo->cmd_pipe_read_fd = fds[0];
    vpninfo->cmd_pipe_write_fd = fds[1];
    return fds[1];
}
#endif

void openconnect_reset_ssl(struct openconnect_info *vpninfo)
{
    (void)vpninfo;
}

int openconnect_make_cstp_connection(struct openconnect_info *vpninfo)
{
    return vpninfo ? 0 : -22;
}

const char *openconnect_get_dtls_cipher(struct openconnect_info *vpninfo)
{
    if (!vpninfo || vpninfo->return_nulls) {
        return NULL;
    }
    return vpninfo->dtls_cipher;
}

const char *openconnect_get_peer_cert_hash(struct openconnect_info *vpninfo)
{
    if (!vpninfo || vpninfo->return_nulls) {
        return NULL;
    }
    return vpninfo->peer_cert_hash;
}

int openconnect_check_peer_cert_hash(struct openconnect_info *vpninfo, const char *old_hash)
{
    if (!vpninfo || !old_hash || !vpninfo->peer_cert_hash) {
        return -1;
    }
    return strcmp(vpninfo->peer_cert_hash, old_hash) == 0 ? 0 : 1;
}

char *openconnect_get_peer_cert_details(struct openconnect_info *vpninfo)
{
    if (!vpninfo || vpninfo->return_nulls) {
        return NULL;
    }
    return strdup("Subject: CN=vpn.example.test\nIssuer: CN=vpn.example.test\n");
}

void openconnect_free_cert_info(struct openconnect_info *vpninfo, void *buf)
{
    (void)vpninfo;
    free(buf);
}

int mock_oc_set_peer_cert_hash(struct openconnect_info *vpninfo, const char *hash)
{
    if (!vpninfo || !hash) {
        return -22;
    }
    free(vpninfo->peer_cert_hash);
    vpninfo->peer_cert_hash = strdup(hash);
    return vpninfo->peer_cert_hash ? 0 : -12;
}

int openconnect_disable_dtls(struct openconnect_info *vpninfo)
{
    if (!vpninfo) {
        return -22;
    }
    vpninfo->dtls_disabled = 1;
    return 0;
}

int openconnect_set_http_proxy(struct openconnect_info *vpninfo, const char *proxy)
{
    if (!vpninfo || !proxy) {
        return -22;
    }
    free(vpninfo->http_proxy);
    vpninfo->http_proxy = strdup(proxy);
    return 0;
}

int openconnect_parse_url(struct openconnect_info *vpninfo, const char *url)
{
    if (!vpninfo || !url) {
        return -22;
    }
    free(vpninfo->hostname);
    vpninfo->hostname = strdup(url);
    return 0;
}

int openconnect_get_port(struct openconnect_info *vpninfo)
{
    return vpninfo ? vpninfo->port : 0;
}

const char *openconnect_get_hostname(struct openconnect_info *vpninfo)
{
    if (!vpninfo || vpninfo->return_nulls) {
        return NULL;
    }
    return vpninfo->hostname;
}

int openconnect_set_client_cert(struct openconnect_info *vpninfo, const char *cert, const char *sslkey)
{
    (void)cert;
    (void)sslkey;
    return vpninfo ? 0 : -22;
}

int openconnect_set_mca_cert(struct openconnect_info *vpninfo, const char *cert, const char *key)
{
    (void)cert;
    (void)key;
    return vpninfo ? 0 : -22;
}

int openconnect_get_ip_info(
    struct openconnect_info *vpninfo,
    const struct oc_ip_info **info,
    const struct oc_vpn_option **cstp_options,
    const struct oc_vpn_option **dtls_options
)
{
    if (!vpninfo) {
        return -22;
    }
    if (info) {
        *info = vpninfo->return_nulls ? NULL : &vpninfo->ip_info;
    }
    if (cstp_options) {
        *cstp_options = NULL;
    }
    if (dtls_options) {
        *dtls_options = NULL;
    }
    return 0;
}

int openconnect_mainloop(
    struct openconnect_info *vpninfo,
    int reconnect_timeout,
    int reconnect_interval
)
{
    (void)reconnect_timeout;
    (void)reconnect_interval;
    if (!vpninfo) {
        return -22;
    }
#ifndef _WIN32
    if (vpninfo->cmd_pipe_read_fd >= 0) {
        unsigned char cmd = 0;
        ssize_t n = read(vpninfo->cmd_pipe_read_fd, &cmd, 1);
        if (n <= 0 || cmd == 'x' /* OC_CMD_CANCEL */ || cmd == 'd' /* OC_CMD_DETACH */) {
            return -2; /* non-zero exit breaks run_loop cleanly */
        }
        return 0;
    }
#endif
    return -2;
}

int openconnect_get_supported_protocols(struct oc_vpn_proto **protos)
{
    struct oc_vpn_proto *arr;
    if (!protos) {
        return -22;
    }
    arr = (struct oc_vpn_proto *)calloc(3, sizeof(struct oc_vpn_proto));
    if (!arr) {
        return -12;
    }
    arr[0].name = "anyconnect";
    arr[0].pretty_name = "Cisco AnyConnect";
    arr[0].description = "Compatible with Cisco AnyConnect SSL VPN";
    arr[0].flags = 0;
    arr[1].name = "gp";
    arr[1].pretty_name = "Palo Alto Networks GlobalProtect";
    arr[1].description = NULL; /* intentional NULL to verify safe null handling */
    arr[1].flags = 0;
    arr[2].name = NULL;
    *protos = arr;
    atomic_fetch_add(&g_live_proto_alloc_count, 1);
    return 2;
}

void openconnect_free_supported_protocols(struct oc_vpn_proto *protos)
{
    if (protos) {
        free(protos);
        atomic_fetch_sub(&g_live_proto_alloc_count, 1);
    }
}

int openconnect_set_option_value(struct oc_form_opt *opt, const char *value)
{
    if (!opt) {
        return -22;
    }
    free(opt->_value);
    opt->_value = value ? strdup(value) : NULL;
    return 0;
}

/* ====== Fault-injection & callback trigger helpers for regression tests ====== */

void mock_oc_trigger_progress_empty(struct openconnect_info *vpninfo)
{
    if (vpninfo && vpninfo->progress) {
        vpninfo->progress(vpninfo->cbdata, 1, "%s", "");
    }
}

void mock_oc_trigger_progress_unknown_level(struct openconnect_info *vpninfo)
{
    if (vpninfo && vpninfo->progress) {
        vpninfo->progress(vpninfo->cbdata, 9999, "unknown level message\n");
    }
}

int mock_oc_trigger_validate_peer_cert(struct openconnect_info *vpninfo, const char *reason)
{
    if (!vpninfo || !vpninfo->validate_peer_cert) {
        return -1;
    }
    return vpninfo->validate_peer_cert(vpninfo->cbdata, reason);
}

void mock_oc_trigger_stats(struct openconnect_info *vpninfo, int include_stats)
{
    if (!vpninfo || !vpninfo->stats_handler) {
        return;
    }
    if (include_stats) {
        struct oc_stats s = { 10, 2048, 20, 4096 };
        vpninfo->stats_handler(vpninfo->cbdata, &s);
    } else {
        vpninfo->stats_handler(vpninfo->cbdata, NULL);
    }
}

void mock_oc_trigger_setup_tun(struct openconnect_info *vpninfo)
{
    if (vpninfo && vpninfo->setup_tun) {
        vpninfo->setup_tun(vpninfo->cbdata);
    }
}

int mock_oc_trigger_auth_form_ignored_opt(struct openconnect_info *vpninfo)
{
    struct oc_form_opt ignored;
    struct oc_auth_form form;
    if (!vpninfo || !vpninfo->process_auth_form) {
        return -1;
    }
    memset(&ignored, 0, sizeof(ignored));
    memset(&form, 0, sizeof(form));
    ignored.flags = 1; /* OC_FORM_OPT_IGNORE */
    ignored.type = 1;  /* OC_FORM_OPT_TEXT */
    ignored.name = "ignored_field";
    form.auth_id = "main";
    form.opts = &ignored;
    return vpninfo->process_auth_form(vpninfo->cbdata, &form);
}

int mock_oc_trigger_auth_form_select(
    struct openconnect_info *vpninfo,
    const char *choice_name,
    const char *choice_label,
    const char *opt_name
)
{
    struct oc_form_opt_select *sel;
    struct oc_choice *ch;
    struct oc_auth_form form;
    int ret;

    if (!vpninfo || !vpninfo->process_auth_form) {
        return -1;
    }

    sel = (struct oc_form_opt_select *)calloc(1, sizeof(*sel));
    ch = (struct oc_choice *)calloc(1, sizeof(*ch));
    ch->name = strdup(choice_name);
    ch->label = strdup(choice_label);

    sel->form.type = 3; /* OC_FORM_OPT_SELECT */
    sel->form.name = strdup(opt_name);
    sel->form._value = NULL;
    sel->nr_choices = 1;
    sel->choices = (struct oc_choice **)calloc(1, sizeof(*sel->choices));
    sel->choices[0] = ch;

    memset(&form, 0, sizeof(form));
    form.auth_id = "main";
    form.opts = &sel->form;

    ret = vpninfo->process_auth_form(vpninfo->cbdata, &form);

    /* Exactly mirrors libopenconnect free_auth_form: frees BOTH _value AND choice->name/label! */
    free(sel->form._value);
    free(sel->form.name);
    free(ch->name);
    free(ch->label);
    free(ch);
    free(sel->choices);
    free(sel);
    return ret;
}

int mock_oc_trigger_auth_form_text_and_password(
    struct openconnect_info *vpninfo,
    const char *text_field_name
)
{
    struct oc_form_opt opt_text;
    struct oc_form_opt opt_pass;
    struct oc_auth_form form;
    int ret;

    if (!vpninfo || !vpninfo->process_auth_form) {
        return -1;
    }
    memset(&opt_text, 0, sizeof(opt_text));
    memset(&opt_pass, 0, sizeof(opt_pass));
    memset(&form, 0, sizeof(form));

    opt_pass.type = 2; /* OC_FORM_OPT_PASSWORD */
    opt_pass.name = "password";

    opt_text.type = 1; /* OC_FORM_OPT_TEXT */
    opt_text.name = (char *)text_field_name;
    opt_text.next = &opt_pass;

    form.auth_id = "main";
    form.opts = &opt_text;

    ret = vpninfo->process_auth_form(vpninfo->cbdata, &form);
    free(opt_text._value);
    free(opt_pass._value);
    return ret;
}

int mock_oc_trigger_auth_form_null_cases(struct openconnect_info *vpninfo)
{
    struct oc_auth_form form;
    struct oc_form_opt opt_null_name;
    int r1, r2, r3;

    if (!vpninfo || !vpninfo->process_auth_form) {
        return -1;
    }

    /* Case 1: NULL form pointer */
    r1 = vpninfo->process_auth_form(vpninfo->cbdata, NULL);

    /* Case 2: NULL auth_id */
    memset(&form, 0, sizeof(form));
    r2 = vpninfo->process_auth_form(vpninfo->cbdata, &form);

    /* Case 3: NULL opt->name */
    memset(&opt_null_name, 0, sizeof(opt_null_name));
    opt_null_name.type = 1; /* OC_FORM_OPT_TEXT */
    opt_null_name.name = NULL;
    form.auth_id = "main";
    form.opts = &opt_null_name;
    r3 = vpninfo->process_auth_form(vpninfo->cbdata, &form);
    free(opt_null_name._value);

    return (r1 < 0 && r2 < 0 && r3 >= 0) ? 0 : -1;
}
