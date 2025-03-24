use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bon::Builder;
use ed25519_compact::PublicKey;
use jsonwebtoken::{jwk::JwkSet, Algorithm, DecodingKey, EncodingKey, TokenData};
use secrecy::{ExposeSecret, SecretString};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use thiserror::Error;

#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::extensions::KeyFormat;

#[cfg_attr(test, automock)]
pub trait TokenManager {
    fn issue<T: Serialize + 'static>(&self, claims: &T) -> Result<String, TokenError>;
    fn validate<T: DeserializeOwned + 'static>(
        &self,
        token: &str,
    ) -> Result<TokenData<T>, TokenError>;

    fn keys(&self) -> Result<JwkSet, TokenError>;
}

#[derive(Builder, Clone)]
pub struct TokenManagerImpl {
    #[builder(into)]
    encoding_key: SecretString,
    #[builder(into)]
    decoding_key: String,
    #[builder(into)]
    audience: String,
    algorithm: Algorithm,
    #[builder(into)]
    kid: String,
    #[builder(into)]
    format: KeyFormat,
    leeway: u64,
}

impl TokenManager for TokenManagerImpl {
    fn issue<T: Serialize>(&self, claims: &T) -> Result<String, TokenError> {
        let encoding_key = self.encoding_key()?;
        let mut header = jsonwebtoken::Header::new(self.algorithm);
        header.kid = Some(self.kid.clone());
        let token =
            jsonwebtoken::encode(&header, claims, &encoding_key).map_err(TokenError::Encode)?;

        Ok(token)
    }

    fn validate<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, TokenError> {
        let decoding_key = self.decoding_key()?;
        let mut validation = jsonwebtoken::Validation::new(self.algorithm);
        validation.leeway = self.leeway;
        validation.set_audience(&[&self.audience]);
        jsonwebtoken::decode::<T>(token, &decoding_key, &validation).map_err(TokenError::Decode)
    }

    fn keys(&self) -> Result<JwkSet, TokenError> {
        match self.algorithm {
            Algorithm::EdDSA => {
                let pk = PublicKey::from_pem(&self.decoding_key).map_err(TokenError::Ed25519)?;

                let x_b64 = URL_SAFE_NO_PAD.encode(pk.as_ref());

                let jwk = json!({
                    "kty": "OKP",                  // Key type for Ed25519
                    "crv": "Ed25519",             // Curve name
                    "x": x_b64,                   // Base64 URL-encoded key
                    "use": "sig",                 // Typically for signing
                    "alg": "EdDSA",                // Algorithm name
                    "kid": self.kid
                });

                Ok(JwkSet {
                    keys: vec![serde_json::from_value(jwk).unwrap()],
                })
            }
            _ => todo!(),
        }
    }
}

impl TokenManagerImpl {
    pub fn audience(&self) -> &str {
        &self.audience
    }

    fn encoding_key(&self) -> Result<EncodingKey, TokenError> {
        match (self.algorithm, &self.format) {
            (Algorithm::EdDSA, KeyFormat::Pem) => {
                EncodingKey::from_ed_pem(self.encoding_key.expose_secret().as_bytes())
                    .map_err(TokenError::Format)
            }
            _ => Err(TokenError::UnsupportedFormat(self.algorithm, self.format)),
        }
    }

    fn decoding_key(&self) -> Result<DecodingKey, TokenError> {
        match (self.algorithm, &self.format) {
            (Algorithm::EdDSA, KeyFormat::Pem) => {
                DecodingKey::from_ed_pem(self.decoding_key.as_bytes()).map_err(TokenError::Format)
            }
            _ => Err(TokenError::UnsupportedFormat(self.algorithm, self.format)),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum TokenError {
    #[error("Error encoding token")]
    Encode(jsonwebtoken::errors::Error),
    #[error("Error decoding token")]
    Decode(jsonwebtoken::errors::Error),
    #[error("Error keys format")]
    Format(jsonwebtoken::errors::Error),
    #[error("Unsupported format: {0:?} {1:?}")]
    UnsupportedFormat(Algorithm, KeyFormat),
    #[error("Error ed25519: {0}")]
    Ed25519(ed25519_compact::Error),
}

#[cfg(test)]
mod tests {
    use crate::{
        extensions::KeyFormat,
        service::token::{TokenError, TokenManager},
    };

    use super::TokenManagerImpl;
    use ed25519_compact::{KeyPair, Seed};
    use jsonwebtoken::{errors::ErrorKind, Algorithm};
    use serde_json::{json, Value};

    fn generate_key_pair() -> (String, String) {
        let key_pair = KeyPair::from_seed(Seed::default());
        (key_pair.sk.to_pem(), key_pair.pk.to_pem())
    }

    fn create_token_manager() -> TokenManagerImpl {
        let (private_key, public_key) = generate_key_pair();

        TokenManagerImpl::builder()
            .encoding_key(private_key)
            .decoding_key(public_key)
            .algorithm(Algorithm::EdDSA)
            .audience("audience")
            .format(KeyFormat::Pem)
            .leeway(0)
            .kid("kid")
            .build()
    }

    #[test]
    fn issue_and_validate() {
        let manager = create_token_manager();
        let exp = chrono::Utc::now();
        let claims = json!({"iss": "test", "aud": "audience", "exp" : exp.timestamp()});

        let token = manager.issue(&claims).unwrap();
        let token_claims = manager.validate::<Value>(&token).unwrap();

        assert_eq!(token_claims.claims, claims);
    }

    #[test]
    fn issue_and_validate_wrong_aud() {
        let manager = create_token_manager();
        let exp = chrono::Utc::now();
        let claims = json!({"iss": "test", "aud": "wrong", "exp" : exp.timestamp()});

        let token = manager.issue(&claims).unwrap();
        let result = manager.validate::<Value>(&token).unwrap_err();

        if let TokenError::Decode(err) = result {
            assert_eq!(err.kind(), &ErrorKind::InvalidAudience);
        } else {
            panic!("Wrong type")
        }
    }
}
