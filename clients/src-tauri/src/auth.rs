pub mod handlers;

use keyring::Entry;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::{Error, Result};

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

impl AuthStore {
    pub(crate) fn new(auth_token: String) -> AuthStore {
        AuthStore {
            auth_token,
            private_key: None,
        }
    }

    pub(crate) fn get_auth_token(&self) -> &str {
        &self.auth_token
    }

    pub(crate) fn with_private_key(&self, private_key: RsaPrivateKey) -> AuthStore {
        AuthStore {
            auth_token: self.auth_token.clone(),
            private_key: Some(private_key),
        }
    }

    pub(crate) fn load(user_id: &Uuid) -> Result<AuthStore> {
        let user_id_str = user_id.to_string();
        let entry = AuthStore::get_token_entry(&user_id_str)?;
        let secret = entry
            .get_password()
            .map_err(|e| Error::Generic(format!("Could not get token: {}", e)))?;
        serde_json::from_str(&secret)
            .map_err(|e| Error::Generic(format!("Could not deserialize auth context: {}", e)))
    }

    pub(crate) fn save(&self, user_id: &Uuid) -> Result<()> {
        let user_id_str = user_id.to_string();
        let entry = AuthStore::get_token_entry(&user_id_str)?;
        let secret = serde_json::to_string(&self)
            .map_err(|e| Error::Generic(format!("Could not serialize auth context: {}", e)))?;
        entry
            .set_password(&secret)
            .map_err(|e| Error::Generic(format!("Could not update token: {}", e)))?;
        Ok(())
    }

    fn get_token_entry(user_id: &str) -> Result<Entry> {
        Entry::new("dev.modzelewski.photo-store", user_id)
            .map_err(|e| Error::Generic(format!("Could not get token entry: {}", e)))
    }
}

impl TryFrom<AuthStore> for AuthCtx {
    type Error = Error;

    fn try_from(store: AuthStore) -> std::result::Result<Self, Self::Error> {
        store
            .private_key
            .map(|private_key| AuthCtx {
                auth_token: store.auth_token,
                private_key,
            })
            .ok_or(Error::Generic(
                "Private key missing, cannot create auth ctx".to_string(),
            ))
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
