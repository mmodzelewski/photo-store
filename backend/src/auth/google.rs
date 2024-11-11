use std::env;

use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use oauth2::basic::{
    BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
    BasicTokenType,
};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, ExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, StandardRevocableToken,
    StandardTokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use dtos::auth::LoginResponse;

use crate::auth::repository::AuthRepository;
use crate::auth::AuthorizationRequest;
use crate::error::Result;
use crate::user::{register_or_get_with_external_provider, AccountProvider};
use crate::AppState;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct IdToken {
    id_token: String,
}

impl ExtraTokenFields for IdToken {}

type GoogleTokenResponse = StandardTokenResponse<IdToken, BasicTokenType>;

type GoogleAuthClient = Client<
    BasicErrorResponse,
    GoogleTokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
>;

#[derive(Clone)]
pub struct GoogleAuth {
    client_id: String,
    client: GoogleAuthClient,
}

impl GoogleAuth {
    pub fn new() -> Self {
        let client_id = env::var("GOOGLE_CLIENT_ID")
            .expect("Missing the GOOGLE_CLIENT_ID environment variable.");
        let google_client_id = ClientId::new(client_id.clone());
        let google_client_secret = ClientSecret::new(
            env::var("GOOGLE_CLIENT_SECRET")
                .expect("Missing the GOOGLE_CLIENT_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .expect("Invalid token endpoint URL");

        let redirect_url =
            RedirectUrl::new("http://localhost:5173/auth/google/redirect".to_string())
                .expect("Invalid redirect URL");

        let client = GoogleAuthClient::new(
            google_client_id,
            Some(google_client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        GoogleAuth { client_id, client }
    }
}

pub(crate) async fn init_authentication(State(state): State<AppState>) -> Result<Redirect> {
    let client = &state.google_auth.client;
    let db = &state.db;

    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    AuthRepository::save_auth_request(
        db,
        AuthorizationRequest {
            state: csrf_state.secret().clone(),
            pkce: pkce_code_verifier.secret().clone(),
        },
    )
    .await?;

    Ok(Redirect::to(authorize_url.as_ref()))
}

#[derive(Deserialize)]
pub(crate) struct AuthResponse {
    code: String,
    state: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: usize,
    iat: usize,
    email: String,
    email_verified: bool,
}

pub(crate) async fn complete_authentication(
    State(state): State<AppState>,
    auth_response: Query<AuthResponse>,
) -> Result<Json<LoginResponse>> {
    let client = &state.google_auth.client;
    let db = &state.db;
    let code = AuthorizationCode::new(auth_response.code.clone());
    let csrf = CsrfToken::new(auth_response.state.clone());
    let auth_request = AuthRepository::get_auth_request_by_state(db, csrf.secret()).await?;

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(auth_request.pkce.clone()))
        .request_async(async_http_client)
        .await
        .unwrap();

    let id_token = &token_response.extra_fields().id_token;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
    validation.set_audience(&[&state.google_auth.client_id]);
    // token comes straight from Google, so we can disable signature validation
    validation.insecure_disable_signature_validation();
    let token_data =
        decode::<Claims>(id_token, &DecodingKey::from_secret(b""), &validation).unwrap();

    let user_id = register_or_get_with_external_provider(
        db,
        &token_data.claims.sub,
        &AccountProvider::Google,
    )
    .await?;
    debug!("User id: {:?}", user_id);

    let auth_token = Uuid::new_v4().to_string();
    AuthRepository::save_auth_token(db, &user_id.0, &auth_token).await?;

    Ok(Json(LoginResponse {
        user_id: user_id.0,
        auth_token,
    }))
}
