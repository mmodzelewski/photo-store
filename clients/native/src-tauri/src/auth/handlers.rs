use anyhow::Context;
use dtos::auth::{PrivateKeyResponse, SaveRsaKeysRequest};
use log::debug;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tiny_http::{Header, Server};
use url::Url;
use uuid::Uuid;

use crate::auth::{AuthCtx, AuthStore};
use crate::http::HttpClient;
use crate::{database::Database, state::SyncedAppState};
use crate::{error::Result, state::User};

#[tauri::command]
pub(crate) async fn authenticate(
    app_handle: AppHandle,
    database: tauri::State<'_, Database>,
    app_state: tauri::State<'_, SyncedAppState>,
) -> Result<()> {
    let server = Server::http("127.0.0.1:0").unwrap();
    let ip = server.server_addr().to_ip().unwrap();

    debug!("Listening on 127.0.0.1:{}", ip.port());
    let redirect_uri = format!("http://127.0.0.1:{}", ip.port());

    app_handle
        .opener()
        .open_url(
            format!(
                "http://localhost:5173/auth/desktop?redirect_uri={}",
                redirect_uri
            ),
            None::<&str>,
        )
        .unwrap();

    if let Ok(request) = server.recv() {
        let url = Url::parse(&format!("http://localhost{}", request.url())).unwrap();

        let (_, auth_token) = url
            .query_pairs()
            .find(|(key, _)| key == "auth_token")
            .unwrap();
        let (_, user_id) = url.query_pairs().find(|(key, _)| key == "user_id").unwrap();
        let user_id = Uuid::parse_str(&user_id).unwrap();

        let auth_store = AuthStore::new(auth_token.to_string());
        auth_store.save(&user_id)?;

        let user = User {
            id: user_id,
            name: "".to_owned(),
        };
        database.save_user(&user)?;
        app_state.replace_user(user);

        let done_url = "http://localhost:5173/auth/desktop/complete";
        let response = tiny_http::Response::empty(303)
            .with_header(Header::from_bytes(&b"Location"[..], done_url.as_bytes()).unwrap());
        request.respond(response).unwrap();

        debug!("Listener closed.");
    }
    app_handle
        .emit("authenticated", ())
        .context("Couldn't emit authenticated message to UI")?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn get_private_key(
    passphrase: String,
    app_state: tauri::State<'_, SyncedAppState>,
    http_client: tauri::State<'_, HttpClient>,
) -> Result<()> {
    debug!("Initiating private key");
    let client = http_client.client();

    let state = app_state.read();
    let user = state.user.context("User is not logged in")?;
    let auth_store = AuthStore::load(&user.id)?;
    let auth_token = auth_store.get_auth_token();

    let private_key = client
        .get(format!("{}/auth/keys", http_client.url()))
        .header("Authorization", auth_token)
        .send()
        .await
        .context("Could not fetch auth keys")?
        .json::<PrivateKeyResponse>()
        .await
        .context("Could not serialize auth keys from json")?;

    let (cipher, nonce) =
        crypto::generate_cipher(&user.id, &passphrase).context("Could not generate cipher")?;
    let private_key = if let Some(private_key_encrypted) = private_key.value {
        debug!("decrypting existing key");
        let pk_der = crypto::decrypt_data_raw(&private_key_encrypted, &cipher, &nonce)
            .context("Could not decrypt private key")?;
        crypto::rsa::from_der(&pk_der).context("Could not create RSA from DER private key")?
    } else {
        debug!("creating new key");
        let private_key = crypto::rsa::generate_key();

        let pk_bytes =
            crypto::rsa::to_der(&private_key).context("Could not serialize private key to der")?;
        let private_key_encrypted = crypto::encrypt_data_raw(&pk_bytes, &cipher, &nonce);
        debug!("new key created");

        let body = SaveRsaKeysRequest {
            private_key: private_key_encrypted.clone(),
            public_key: crypto::rsa::to_public_key_pem(&private_key)
                .context("Could not get public key from private key")?,
        };
        client
            .post(format!("{}/auth/keys", http_client.url()))
            .header("Content-Type", "application/json")
            .header("Authorization", auth_token)
            .body(serde_json::to_string(&body).unwrap())
            .send()
            .await
            .context("Failed sending keys to backend")?;
        debug!("key sent");
        private_key
    };

    let auth_store = auth_store.with_private_key(private_key);
    auth_store.save(&user.id)?;
    let ctx: AuthCtx = auth_store.try_into()?;
    app_state.replace_auth_ctx(ctx);

    Ok(())
}
