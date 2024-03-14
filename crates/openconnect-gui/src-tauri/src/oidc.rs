use anyhow::Ok;
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::{reqwest, TokenResponse};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

pub struct OpenID {
    client: CoreClient,
}

impl OpenID {
    pub fn new(
        issuer_url: String,
        redirect_uri: String,
        client_id: String,
        client_secret: Option<String>,
    ) -> anyhow::Result<Self> {
        let issuer_url = IssuerUrl::new(issuer_url)?;
        let provider_metadata =
            CoreProviderMetadata::discover(&issuer_url, openidconnect::reqwest::http_client)?;
        let redirect_uri = RedirectUrl::new(redirect_uri)?;
        let client_id = ClientId::new(client_id);
        let client_secret = client_secret.map(ClientSecret::new);

        let client =
            CoreClient::from_provider_metadata(provider_metadata, client_id, client_secret)
                .set_redirect_uri(redirect_uri);

        Ok(OpenID {
            // issuer_url,
            // redirect_uri,
            // client_id,
            // client_secret,
            client,
        })
    }

    pub fn auth_request(&self) -> anyhow::Result<(Url, CsrfToken, Nonce)> {
        let (authorize_url, csrf_state, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .url();

        Ok((authorize_url, csrf_state, nonce))
    }

    pub fn parse_code_and_state(&self, url: Url) -> (AuthorizationCode, CsrfToken) {
        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
            .unwrap();

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))
            .unwrap();

        (code, state)
    }

    pub fn exchange_token(&self, code: AuthorizationCode) -> anyhow::Result<String> {
        let token_response = self
            .client
            .exchange_code(code)
            .request(reqwest::http_client)?;

        let token = token_response.id_token().unwrap().to_string();

        Ok(token)
    }

    pub fn wait_for_callback(&self) -> anyhow::Result<(AuthorizationCode, CsrfToken)> {
        Ok({
            // A very naive implementation of the redirect server.
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

            // Accept one connection
            let (mut stream, _) = listener.accept().unwrap();

            let mut reader = BufReader::new(&stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();

            let redirect_url = request_line.split_whitespace().nth(1).unwrap();
            let url =
                Url::parse(&(env::var("OIDC_REDIRECT_URI").unwrap() + redirect_url)).unwrap();

            let (code, state) = self.parse_code_and_state(url);

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            (code, state)
        })
    }
}

#[cfg(test)]
mod test {
    use openidconnect::AdditionalClaims;
    use serde::{Deserialize, Serialize};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use url::Url;

    use crate::oidc::OpenID;

    #[derive(Debug, Deserialize, Serialize)]
    struct GitLabClaims {
        // Deprecated and thus optional as it might be removed in the futre
        sub_legacy: Option<String>,
        groups: Vec<String>,
    }
    impl AdditionalClaims for GitLabClaims {}

    #[test]
    fn test() {
        let openid = OpenID::new(
            env::var("OIDC_ISSUER").unwrap(),
            env::var("OIDC_REDIRECT_URI").unwrap(),
            env::var("OIDC_CLIENT_ID").unwrap(),
            Some(env::var("OIDC_CLIENT_SECRET")),
        )
        .unwrap();

        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state, _nonce) = openid.auth_request().unwrap();

        println!("Open this URL in your browser:\n{authorize_url}\n");

        let (code, state) = {
            // A very naive implementation of the redirect server.
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

            // Accept one connection
            let (mut stream, _) = listener.accept().unwrap();

            let mut reader = BufReader::new(&stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();

            let redirect_url = request_line.split_whitespace().nth(1).unwrap();
            let url =
                Url::parse(&(env::var("OIDC_REDIRECT_URI").unwrap() + redirect_url)).unwrap();

            let (code, state) = openid.parse_code_and_state(url);

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            (code, state)
        };

        println!("GitLab returned the following code:\n{}\n", code.secret());
        println!(
            "GitLab returned the following state:\n{} (expected `{}`)\n",
            state.secret(),
            csrf_state.secret()
        );

        let token = openid.exchange_token(code).unwrap();
        println!("Token: {}", token);

        // Exchange the code with a token.
        // let token_response = openid
        //     .client
        //     .exchange_code(code)
        //     .request(reqwest::http_client)
        //     .unwrap_or_else(|err| {
        //         handle_error(&err, "No user info endpoint");
        //         unreachable!();
        //     });

        // println!(
        //     "GitLab returned access token:\n{}\n",
        //     token_response.access_token().secret()
        // );
        // println!("GitLab returned scopes: {:?}", token_response.scopes());

        // let id_token_verifier: CoreIdTokenVerifier = openid.client.id_token_verifier();
        // let id_token_claims: &CoreIdTokenClaims = token_response
        //     .extra_fields()
        //     .id_token()
        //     .expect("Server did not return an ID token")
        //     .claims(&id_token_verifier, &nonce)
        //     .unwrap_or_else(|err| {
        //         handle_error(&err, "Failed to verify ID token");
        //         unreachable!();
        //     });
        // println!("GitLab returned ID token: {:?}\n", id_token_claims);

        // let userinfo_claims: UserInfoClaims<GitLabClaims, CoreGenderClaim> = openid
        //     .client
        //     .user_info(token_response.access_token().to_owned(), None)
        //     .unwrap_or_else(|err| {
        //         handle_error(&err, "No user info endpoint");
        //         unreachable!();
        //     })
        //     .request(reqwest::http_client)
        //     .unwrap_or_else(|err| {
        //         handle_error(&err, "Failed requesting user info");
        //         unreachable!();
        //     });
        // println!("GitLab returned UserInfo: {:?}", userinfo_claims);
    }
}
