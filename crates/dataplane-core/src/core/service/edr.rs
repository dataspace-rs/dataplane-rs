use bon::Builder;
use chrono::{Duration, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    core::model::{
        edr::{Edr, EdrClaims, RefreshTokenId, TokenId},
        namespace::{EDC_NAMESPACE, IDSA_NAMESPACE},
        token::{TokenRequest, TokenResponse},
    },
    signaling::{DataAddress, DataFlowStartMessage, EndpointProperty},
};

use super::token::{TokenError, TokenManager};

#[derive(Clone, Builder)]
pub struct EdrManager<T: TokenManager> {
    #[builder(into)]
    pub(crate) proxy_url: String,
    #[builder(into)]
    pub(crate) token_url: String,
    #[builder(into)]
    issuer: String,
    #[builder(into)]
    pub(crate) jwks_url: String,
    pub(crate) tokens: T,
    token_duration: Duration,
    #[builder(default = Duration::days(30))]
    refresh_token_duration: Duration,
}

impl<T: TokenManager> EdrManager<T> {
    pub async fn create_edr(&self, req: &DataFlowStartMessage) -> Result<Edr, EdrError> {
        let token_id: TokenId = Uuid::new_v4().into();
        let refresh_token_id: RefreshTokenId = Uuid::new_v4().into();

        let data_address = DataAddress::builder()
            .endpoint_type(IDSA_NAMESPACE.to_iri("HTTP"))
            .endpoint_properties(self.endpoint_properties(token_id, refresh_token_id, req)?)
            .build();

        Ok(Edr::builder()
            .token_id(token_id)
            .refresh_token_id(refresh_token_id)
            .data_address(data_address)
            .build())
    }

    pub async fn refresh_token(&self, req: TokenRequest) -> Result<TokenResponse, EdrError> {
        let token_id: TokenId = Uuid::new_v4().into();
        let refresh_token_id: RefreshTokenId = Uuid::new_v4().into();

        let claims = self.tokens.validate::<EdrClaims>(&req.refresh_token)?;

        self.issue_token(
            token_id,
            refresh_token_id,
            &claims.claims.sub,
            &claims.claims.transfer_id,
        )
    }

    fn endpoint_properties(
        &self,
        token_id: TokenId,
        refresh_token_id: RefreshTokenId,
        req: &DataFlowStartMessage,
    ) -> Result<Vec<EndpointProperty>, EdrError> {
        let token_response = self.issue_token(
            token_id,
            refresh_token_id,
            &req.participant_id,
            &req.process_id,
        )?;
        Ok(vec![
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("endpoint"))
                .value(self.proxy_url.clone())
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("access_token"))
                .value(token_response.access_token)
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("token_type"))
                .value("Bearer")
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("refresh_token"))
                .value(token_response.refresh_token)
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("refresh_endpoint"))
                .value(self.token_url.clone())
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("expires_in"))
                .value(token_response.expires_in)
                .build(),
            EndpointProperty::builder()
                .name(EDC_NAMESPACE.to_iri("jwks_url"))
                .value(self.jwks_url.clone())
                .build(),
        ])
    }

    fn issue_access_token(
        &self,
        id: TokenId,
        participant_id: &str,
        process_id: &str,
    ) -> Result<String, EdrError> {
        self.issue_generic_token(id.into(), participant_id, process_id, self.token_duration)
    }

    fn issue_generic_token(
        &self,
        jti: Uuid,
        participant_id: &str,
        process_id: &str,
        duration: Duration,
    ) -> Result<String, EdrError> {
        let now = Utc::now();
        let exp = now
            .checked_add_signed(duration)
            .ok_or_else(|| anyhow::anyhow!("Error adding {}", self.token_duration))
            .map_err(EdrError::Generic)?;

        let claims = EdrClaims::builder()
            .jti(jti)
            .iss(self.issuer.clone())
            .aud(self.proxy_url.clone())
            .sub(participant_id.to_string())
            .exp(exp.timestamp())
            .iat(now.timestamp())
            .transfer_id(process_id.to_string())
            .build();

        self.tokens.issue(&claims).map(Ok)?
    }

    pub(crate) fn issue_token(
        &self,
        token_id: TokenId,
        refresh_token_id: RefreshTokenId,
        participant_id: &str,
        process_id: &str,
    ) -> Result<TokenResponse, EdrError> {
        let access_token = self.issue_access_token(token_id, participant_id, process_id)?;
        let refresh_token =
            self.issue_refresh_token(refresh_token_id, participant_id, process_id)?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.token_duration.num_seconds().to_string(),
        })
    }

    fn issue_refresh_token(
        &self,
        id: RefreshTokenId,
        participant_id: &str,
        process_id: &str,
    ) -> Result<String, EdrError> {
        self.issue_generic_token(
            id.into(),
            participant_id,
            process_id,
            self.refresh_token_duration,
        )
    }
}

#[derive(Error, Debug)]
pub enum EdrError {
    #[error("Generic error")]
    Generic(anyhow::Error),
    #[error(transparent)]
    Token(#[from] TokenError),
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;
    use crate::core::model::namespace::IDSA_NAMESPACE;
    use crate::core::service::token::MockTokenManager;
    use crate::signaling::{DataFlowStartMessage, FlowType};
    use chrono::Duration;
    use jsonwebtoken::errors::ErrorKind;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_edr() {
        let mut token_manager = MockTokenManager::new();

        token_manager
            .expect_issue::<EdrClaims>()
            .withf(|claims| {
                claims.iss == "issuer".to_string()
                    && claims.aud == "http://localhost:8080/public".to_string()
                    && claims.sub == "participant_id".to_string()
                    && claims.transfer_id == "process_id".to_string()
            })
            .returning(|_: &EdrClaims| Ok("token".to_string()));

        let edr_manager = EdrManager::builder()
            .proxy_url("http://localhost:8080/public")
            .issuer("issuer")
            .tokens(token_manager)
            .token_duration(Duration::hours(1))
            .token_url("http://localhost:8080/token")
            .jwks_url("http://localhost:8080/.well-known/jwks.json")
            .build();

        let req = create_req();

        let edr = edr_manager.create_edr(&req).await.unwrap();

        assert_eq!(
            edr.data_address.endpoint_type,
            IDSA_NAMESPACE.to_iri("HTTP")
        );
        assert_eq!(edr.data_address.endpoint_properties.len(), 7);

        assert_eq!(
            edr.data_address
                .get_property(&EDC_NAMESPACE.to_iri("access_token")),
            Some("token")
        );

        assert_eq!(
            edr.data_address
                .get_property(&EDC_NAMESPACE.to_iri("refresh_token")),
            Some("token")
        );

        assert_eq!(
            edr.data_address
                .get_property(&EDC_NAMESPACE.to_iri("endpoint")),
            Some(edr_manager.proxy_url.as_ref())
        );

        assert_eq!(
            edr.data_address
                .get_property(&EDC_NAMESPACE.to_iri("jwks_url")),
            Some(edr_manager.jwks_url.as_ref())
        );

        assert_eq!(
            edr.data_address
                .get_property(&EDC_NAMESPACE.to_iri("expires_in")),
            Some("3600")
        );
    }

    #[tokio::test]
    async fn test_create_edr_failure() {
        let mut token_manager = MockTokenManager::new();

        token_manager
            .expect_issue::<EdrClaims>()
            .returning(|_: &EdrClaims| Err(TokenError::Encode(ErrorKind::InvalidKeyFormat.into())));

        let edr_manager = EdrManager::builder()
            .proxy_url("http://localhost:8080/public".to_string())
            .issuer("issuer".to_string())
            .tokens(token_manager)
            .token_duration(Duration::days(1))
            .token_url("http://localhost:8080/token")
            .jwks_url("http://localhost:8080/.well-known/jwks.json")
            .build();

        let req = create_req();

        let result = edr_manager.create_edr(&req).await;

        if let Err(EdrError::Token(TokenError::Encode(err))) = result {
            assert_eq!(err.kind(), &ErrorKind::InvalidKeyFormat);
        } else {
            panic!("Wrong type")
        }
    }

    fn create_req() -> DataFlowStartMessage {
        DataFlowStartMessage::builder()
            .participant_id("participant_id".to_string())
            .process_id("process_id".to_string())
            .source_data_address(
                DataAddress::builder()
                    .endpoint_type("MyType".to_string())
                    .endpoint_properties(vec![])
                    .build(),
            )
            .properties(HashMap::new())
            .flow_type(FlowType::Pull)
            .dataset_id(Uuid::new_v4().to_string())
            .agreement_id(Uuid::new_v4().to_string())
            .build()
    }
}
