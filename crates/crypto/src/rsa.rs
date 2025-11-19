use rsa::{
    RsaPrivateKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, der::zeroize::Zeroizing},
};

use crate::error::{Error, Result};

pub fn generate_key() -> RsaPrivateKey {
    let mut rng = rand::thread_rng();

    let bits = 2048;
    RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key")
}

pub fn from_der(pk_der: &[u8]) -> Result<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_der(pk_der)
        .map_err(|e| Error::EncryptionError(format!("Could not parse private key: {}", e)))
}

pub fn to_der(private_key: &RsaPrivateKey) -> Result<Zeroizing<Vec<u8>>> {
    private_key
        .to_pkcs8_der()
        .map_err(|e| {
            Error::EncryptionError(format!(
                "Could not convert private key to DER format: {}",
                e
            ))
        })
        .map(|der| der.to_bytes())
}

pub fn to_public_key_pem(private_key: &RsaPrivateKey) -> Result<String> {
    private_key
        .to_public_key()
        .to_public_key_pem(base64ct::LineEnding::LF)
        .map_err(|e| {
            Error::EncryptionError(format!(
                "Could not convert private key to public key PEM format: {}",
                e
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_have_the_same_key_after_converting_to_der_and_back() {
        let private_key = generate_key();
        let der = to_der(&private_key).unwrap();
        let private_key2 = from_der(&der).unwrap();
        assert_eq!(private_key, private_key2);
    }

    #[test]
    fn should_convert_private_key_to_public_key_pem() {
        let private_key = RsaPrivateKey::from_pkcs8_pem("-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDZO3b745VAer97\n9trdfmaDuggrt3aknjkf4+V3SAGISBXwvQrLtlPoWmtBhwBck/Yia9vG7irQ+u0Z\npi7Lx6cIOXfd2UCM0YzmQ5StpNSF/kLrOBBuvC1YQe6aKGxB14+P7SHklpwi8MO2\nA8O4PeDJTdXq6XVVCpkyWwUbQ63oYJvLI/i08OhXXRu6G49dsAPtHBjHRCNUAOaY\ntwP8myDnwvUvHjbm5tZipHjcYOWjqlWim+Jqd2jsBQufFAwW/nnhWtKeSGa3ljL1\no6dP4oafKksZKw/ayIqE2UGB6nUBg5vkw54ooIfLKXXW4ACu6x53GQLvYZcEqfDM\na5/bKA1xAgMBAAECggEBAJAuQbjJwsQ7NGCo5Xdhb9U6YjXx3RNB2RRrhF/5MNst\nTTKtpj6zU1nCubGSUxEfO5x5DjQo28483aXKgQDMEPcKfZ6HlaphYy1p6YKfBlew\n/OV2HqIAz+/mQuGats+0rRqP/5Dizdr7BksGkJ72ov25ZaQ3M6MwF6Iue2MvNnwm\neP2JneNc1tkj3Cc6dVMrRpAVDv0uEFdhiiVErRDfrX8MflugxpCxFRQ7w3l1AOsl\nygTDE2VbzKh5Bqwf/5n1/np/r5LIVX1hwOvPC/Ku07h0K3JfEiHfqq9U/f6FV+Nr\n3Rc9OemhJiD3d0FrmYNtrbiyD5qN7T8xt5+2kF1Ek+kCgYEA2fN39e/fvB3QXCOR\ncOKyuz+MhxK3zxgvVsIStIWzGLoJOUU/sBEethJKupQ0aMXtTQ98FLvHaZMtJkU3\nLxTxv8MdnZdGtnTos000nyeAds3FSvW8VqFknUqgeKQJv2XXjk2rhqjJIbz334ey\nIL6hK2iRL3eGJOeSbOzdo63lEVcCgYEA/yffo+qEW/FzSukwgXxQ4XP+Je6GMwgr\nbwFdTs7XL5slRjTObF2zF+nk0LgoMHM+wEOqdfoGQLI1QQag5LfQh4Bbul/XHMme\n+M3aUqTgcOIM9+hUOY3YkhHQp1K9oWS0XzaVqoiLoeKP8H2oD3lo9VaLcxwnoGmt\nhv7vILnGMncCgYBv7iqIYnV7jbAo8ZdK6xGxOlS4NbOyJpCBNNAYZ6VvqHL+N4Ma\nr+Aez0wTf/Neb2+MKMyndTxXCt+gDOHnSxFQUysNeNg28dlj492Hcuj1mn2dHpBn\nySD76ox5CH19DxdhnJ/fWyVYL0z5Ph4L1Pq+aUhOoUqB/29ig07cNX0zpwKBgQCr\n+Kjv5qFdAsiNJcwOicZNngseb7w9avUzNP05n4lDSdL+lZrHSQPrSzZwQp67wQD7\nPuAF8gUC19mywQ/x973xhd7NJ8lpWq7tzHiomP24t3K8J/eUbvkXwjAahlbPD3vO\nbJDFRpCuBHC1S0vZWiAWs0T7yW8f2/ob8XkkWnGuEwKBgF03Al3md8kDwAVU2grV\n7ZbWNSZnpQNMx2S4takXQcVF/+OqoLgAkQENiTuSXTqPALt5DUapYcoo12LNrBQX\n+TlgTBNYP1j1AWifkWvfP01PiySAJV0hq0/FonB3+lKRpIgYLQhPzd2LTJm5yr+W\nZxiJhkoW0ghGRuM3DgMXu2ot\n-----END PRIVATE KEY-----\n").unwrap();
        let pem = to_public_key_pem(&private_key).unwrap();
        assert!(pem.eq("-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2Tt2++OVQHq/e/ba3X5m\ng7oIK7d2pJ45H+Pld0gBiEgV8L0Ky7ZT6FprQYcAXJP2Imvbxu4q0PrtGaYuy8en\nCDl33dlAjNGM5kOUraTUhf5C6zgQbrwtWEHumihsQdePj+0h5JacIvDDtgPDuD3g\nyU3V6ul1VQqZMlsFG0Ot6GCbyyP4tPDoV10buhuPXbAD7RwYx0QjVADmmLcD/Jsg\n58L1Lx425ubWYqR43GDlo6pVopviando7AULnxQMFv554VrSnkhmt5Yy9aOnT+KG\nnypLGSsP2siKhNlBgep1AYOb5MOeKKCHyyl11uAArusedxkC72GXBKnwzGuf2ygN\ncQIDAQAB\n-----END PUBLIC KEY-----\n"));
    }
}
