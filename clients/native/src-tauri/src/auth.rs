pub mod handlers;

use crate::{Error, Result};
use anyhow::{Context, anyhow};
use crypto::rsa::{from_der, to_der};
use keyring::Entry;
use log::trace;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub(crate) struct AuthStore {
    auth_token: String,
    private_key: Option<RsaPrivateKey>,
}

#[derive(Clone)]
pub(crate) struct AuthCtx {
    auth_token: String,
    private_key: RsaPrivateKey,
}

const TOKEN_ENTRY: &str = "dev.modzelewski.photo-store.token";
const PRIVATE_KEY_ENTRY: &str = "dev.modzelewski.photo-store.private-key";

impl AuthStore {
    pub(crate) fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            private_key: None,
        }
    }

    pub(crate) fn get_auth_token(&self) -> &str {
        &self.auth_token
    }

    pub(crate) fn with_private_key(&self, private_key: RsaPrivateKey) -> Self {
        Self {
            auth_token: self.auth_token.clone(),
            private_key: Some(private_key),
        }
    }

    pub(crate) fn load(user_id: &Uuid) -> Result<AuthStore> {
        let user_id_str = user_id.to_string();
        let token_entry = AuthStore::get_entry(TOKEN_ENTRY, &user_id_str)?;
        let token = token_entry.get_password().context("Could not get token")?;

        let pk_entry = AuthStore::get_entry(PRIVATE_KEY_ENTRY, &user_id_str)?;
        let pk_der: Option<Vec<u8>> = match pk_entry.get_secret() {
            Ok(pk_der) => Some(pk_der),
            Err(keyring::Error::NoEntry) => None,
            Err(err) => Err(anyhow!("Could not get private key from keyring: {}", err))?,
        };

        let private_key = if let Some(pk_der) = pk_der {
            Some(
                from_der(&pk_der)
                    .map_err(|err| anyhow!("Could not convert private key from der : {}", err))?,
            )
        } else {
            None
        };

        Ok(Self {
            auth_token: token,
            private_key,
        })
    }

    pub(crate) fn save(&self, user_id: &Uuid) -> Result<()> {
        let user_id_str = user_id.to_string();
        let token_entry = AuthStore::get_entry(TOKEN_ENTRY, &user_id_str)?;
        token_entry
            .set_password(&self.auth_token)
            .map_err(|err| anyhow!("Could not save auth token to keyring: {}", err))?;
        trace!("Auth token saved");

        if let Some(private_key) = &self.private_key {
            let pk_entry = AuthStore::get_entry(PRIVATE_KEY_ENTRY, &user_id_str)?;
            let pk_der = to_der(private_key)
                .map_err(|err| anyhow!("Could not convert private key to der: {}", err))?;
            pk_entry
                .set_secret(&pk_der)
                .map_err(|err| anyhow!("Could not save private key to keyring: {}", err))?;
            trace!("Private key saved");
        }

        Ok(())
    }

    fn get_entry(key: &str, user_id: &str) -> Result<Entry> {
        let entry = Entry::new(key, user_id)
            .map_err(|err| anyhow!("Could not get keyring entry ({key}): {err}"))?;
        Ok(entry)
    }
}

impl TryFrom<AuthStore> for AuthCtx {
    type Error = Error;

    fn try_from(store: AuthStore) -> Result<Self> {
        let ctx = store
            .private_key
            .map(|private_key| AuthCtx {
                auth_token: store.auth_token,
                private_key,
            })
            .context("Private key missing, cannot create auth ctx")?;
        Ok(ctx)
    }
}

impl AuthCtx {
    pub(crate) fn get_public_key(&self) -> RsaPublicKey {
        RsaPublicKey::from(&self.private_key)
    }

    pub(crate) fn decrypt(
        &self,
    ) -> impl Fn(&[u8]) -> std::result::Result<Vec<u8>, crypto::error::Error> + '_ {
        |data| {
            let padding = Oaep::new::<Sha256>();

            self.private_key.decrypt(padding, data).map_err(|e| {
                crypto::error::Error::EncryptionError(format!("Could not decrypt data: {}", e))
            })
        }
    }

    pub(crate) fn get_auth_token(&self) -> &str {
        &self.auth_token
    }
}
