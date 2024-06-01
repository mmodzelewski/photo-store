use keyring::Entry;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::{Error, Result};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AuthCtx {
    auth_token: String,
    private_key: RsaPrivateKey,
}

impl AuthCtx {
    pub(crate) fn new(auth_token: String, private_key: RsaPrivateKey) -> AuthCtx {
        AuthCtx {
            auth_token,
            private_key,
        }
    }

    pub(crate) fn load(user_id: &Uuid) -> Result<AuthCtx> {
        let user_id_str = user_id.to_string();
        let entry = AuthCtx::get_token_entry(&user_id_str)?;
        let secret = entry
            .get_password()
            .map_err(|e| Error::Generic(format!("Could not get token: {}", e)))?;
        serde_json::from_str(&secret)
            .map_err(|e| Error::Generic(format!("Could not deserialize auth context: {}", e)))
    }

    pub(crate) fn save(&self, user_id: &Uuid) -> Result<()> {
        let user_id_str = user_id.to_string();
        let entry = AuthCtx::get_token_entry(&user_id_str)?;
        let secret = serde_json::to_string(&self)
            .map_err(|e| Error::Generic(format!("Could not serialize auth context: {}", e)))?;
        entry
            .set_password(&secret)
            .map_err(|e| Error::Generic(format!("Could not update token: {}", e)))?;
        Ok(())
    }

    fn get_token_entry(user_id: &str) -> Result<keyring::Entry> {
        return Entry::new("dev.modzelewski.photo-store", user_id)
            .map_err(|e| Error::Generic(format!("Could not get token entry: {}", e)));
    }

    pub(crate) fn get_public_key(&self) -> rsa::RsaPublicKey {
        RsaPublicKey::from(&self.private_key)
    }

    pub(crate) fn decrypt(
        &self,
    ) -> impl Fn(&[u8]) -> std::result::Result<Vec<u8>, crypto::error::Error> + '_ {
        |data| {
            let padding = Oaep::new::<Sha256>();

            self.private_key.decrypt(padding, &data).map_err(|e| {
                crypto::error::Error::EncryptionError(format!("Could not decrypt data: {}", e))
            })
        }
    }

    pub(crate) fn get_auth_token(&self) -> &str {
        &self.auth_token
    }
}
