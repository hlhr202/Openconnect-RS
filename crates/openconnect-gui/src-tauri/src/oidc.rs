use tokio::test;

#[test]
async fn test_oidc() {
    let client = reqwest::Client::new();
    let url = "http://10.0.3.12:9001/oauth2/authorize";

    let params = [
        ("client_id", "oidc-client"),
        ("response_type", "code"),
        (
            "redirect_uri",
            "https://10.0.3.12:9001/login/oauth2/code/oidc-client",
        ),
        ("scope", "openid profile"),
        ("state", "txy"),
        (
            "code_challenge",
            "ijLN8kgcBV5l9RDw5dh6YDzuidgNHtvkVLEwsnjlFUc",
        ),
        ("code_challenge_method", "S256"),
        ("nonce", "123yyy"),
    ];

    let url = reqwest::Url::parse_with_params(url, &params).unwrap();

    println!("Request: {:?}", url.to_string());

    let res = client.get(url).query(&params).send().await.unwrap();

    println!("Response: {:?}", res.text().await.unwrap());
}

#[tokio::test]
async fn test_auth_token() {
    let url = "http://10.0.3.12:9001/oauth2/token";
    let payload = [
        ("grant_type", "authorization_code"),
        ("code", "94xIdNfju-8yKY9a_9xx5rGl4kvH-oUYlJ40LlaRF4qsvwUFkMgOoc11DjLWsNAZYXfU6nLUu7ksvT0T4HrLiEh2MPg2F3XuxGCtwQkJOmMLih3CaaU3U30oQT2tOpeW"),
        ("redirect_uri", "http://localhost:3001"),
        ("code_verifier", "rkTspoTlm1qPeMGIHydKQoNRoeXuDewb-LxyJeUr4cM35I9wicDKxB7TlCus51cMPd4_hEBMyYdChzmCoBEbKR25BC2pK66wDv9FGC3H98BS_Jny_GEO_gjipDbFH_Lu")
    ];

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .basic_auth("oidc-client", Some("123456"))
        .form(&payload)
        .send()
        .await
        .unwrap();

    println!("Response: {:?}", res.text().await.unwrap());
}
