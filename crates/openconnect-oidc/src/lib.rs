use std::str::FromStr;

pub mod oidc_device;
pub mod oidc_token;

pub async fn obtain_cookie_by_oidc_token(server_url: &str, token: &str) -> Option<String> {
    let client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .default_headers(reqwest::header::HeaderMap::new())
        .http1_allow_obsolete_multiline_headers_in_responses(true)
        .http1_allow_spaces_after_header_name_in_responses(true)
        .http1_ignore_invalid_headers_in_responses(true)
        .http1_title_case_headers()
        .no_brotli()
        .no_deflate()
        .no_gzip()
        .no_proxy()
        .build()
        .ok()?;

    let mut url = reqwest::Url::from_str(server_url).ok()?;
    if url.path().is_empty() {
        url.set_path("auth")
    }
    let req_builder = client
        .post(url)
        .header("Accept", "*/*")
        .header("User-Agent", "AnyConnect Compatible Client")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .bearer_auth(token);

    let req = req_builder.build().ok()?;

    let res = client.execute(req).await.ok()?;
    if !res.status().is_success() {
        eprintln!(
            "Failed to obtain cookie from server. Error code: {:?}",
            res.status()
        );
        return None;
    }

    let combined_cookie = res
        .cookies()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; ");

    Some(combined_cookie)
}