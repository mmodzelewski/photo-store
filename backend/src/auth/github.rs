use axum::extract::Query;

use oauth2::basic::BasicClient;

use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::{dbg, env};

pub async fn setup_github() -> CsrfToken {
    let github_client_id = ClientId::new(
        env::var("GITHUB_CLIENT_ID").expect("Missing the GITHUB_CLIENT_ID environment variable."),
    );
    let github_client_secret = ClientSecret::new(
        env::var("GITHUB_CLIENT_SECRET")
            .expect("Missing the GITHUB_CLIENT_SECRET environment variable."),
    );
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
    // This example will be running its own server at localhost:8080.
    // See below for the server implementation.
    .set_redirect_uri(
        RedirectUrl::new("http://127.0.0.1:3000/redirect".to_string())
            .expect("Invalid redirect URL"),
    );

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    println!(
        "Open this URL in your browser:\n{}\n",
        authorize_url.to_string()
    );

    return csrf_state;
}

#[derive(Deserialize, Debug)]
pub struct GithubCodeCallback {
    code: String,
    state: String,
}

pub async fn redirect(Query(query): Query<GithubCodeCallback>) {
    let github_client_id = ClientId::new(
        env::var("GITHUB_CLIENT_ID").expect("Missing the GITHUB_CLIENT_ID environment variable."),
    );
    let github_client_secret = ClientSecret::new(
        env::var("GITHUB_CLIENT_SECRET")
            .expect("Missing the GITHUB_CLIENT_SECRET environment variable."),
    );
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("Invalid token endpoint URL");
    let code = AuthorizationCode::new(query.code);
    dbg!(&code);
    let csrf = CsrfToken::new(query.state);
    dbg!(&csrf);
    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
    // This example will be running its own server at localhost:8080.
    // See below for the server implementation.
    .set_redirect_uri(
        RedirectUrl::new("http://127.0.0.1:3000/redirect".to_string())
            .expect("Invalid redirect URL"),
    );

    let auth_token = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;

    match auth_token {
        Ok(token) => {
            dbg!(&token);
            dbg!(&token.extra_fields());
            dbg!(token.access_token().secret());
            let scopes = if let Some(scopes_vec) = token.scopes() {
                scopes_vec
                    .iter()
                    .map(|comma_separated| comma_separated.split(','))
                    .flatten()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            println!("Github returned the following scopes:\n{:?}\n", scopes);
            let r_client = reqwest::Client::new();
            let request = r_client
                .get("https://api.github.com/user")
                //.bearer_auth(token.access_token().secret())
                .header(
                    "Authorization",
                    format!("Bearer {}", token.access_token().secret()),
                );
            dbg!(&request);
            let user = request.send().await.unwrap();

            dbg!(&user);
        }
        Err(err) => {
            println!("Token exchange failed: {:?}", err);
        }
    }
}

#[derive(Deserialize, Debug)]
struct EmailAddress {
    email: String,
    primary: bool,
    verified: bool,
    visibility: Option<String>,
}

pub async fn get_user_data() {
    let r_client = reqwest::Client::new();
    let request = r_client
        .get("https://api.github.com/user/emails")
        .header(reqwest::header::USER_AGENT, "mmodzelewski")
        .bearer_auth("<token>");
    dbg!(&request);
    let user = request.send().await;
    match user {
        Ok(user) => {
            dbg!(&user);
            let user = user.json::<Vec<EmailAddress>>().await.unwrap();
            dbg!(user);
        }
        Err(err) => {
            dbg!(&err);
        }
    }
}
