use axum::extract::Request;
use axum::RequestPartsExt;
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::request::Parts,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::errors::ErrorKind;
use reqwest::StatusCode;
use serde_json::json;
use tracing::debug;

use crate::core::model::edr::EdrClaims;
use crate::core::model::namespace::EDC_NAMESPACE;
use crate::core::model::transfer::{Transfer, TransferStatus};
use crate::core::service::token::TokenManager;
use crate::web::state::Context;

pub async fn public_handler<T: TokenManager + Clone>(
    claims: EdrClaims,
    State(ctx): State<Context<T>>,
    req: Request,
) -> Result<Response, PublicError> {
    let transfer = ctx
        .transfer_manager()
        .get(claims.transfer_id())
        .await?
        .filter(|transfer| {
            transfer.status == TransferStatus::Started && transfer.token_id == claims.jti.into()
        })
        .ok_or_else(|| {
            debug!(
                "Transfer with id {:#} not found or not active",
                claims.transfer_id()
            );
            PublicError::TransferNotValid
        })?;

    proxy(req, transfer).await
}

async fn proxy(request: Request, transfer: Transfer) -> Result<Response, PublicError> {
    let property = EDC_NAMESPACE.to_iri("baseUrl");
    let url = transfer
        .source
        .get_property(&property)
        .ok_or_else(|| anyhow::anyhow!("Property {} not found", property))?;

    debug!("Proxying request to {}", url);
    let client = reqwest::Client::new();
    let response = client.request(request.method().clone(), url).send().await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await?;

    let mut response = Response::builder().status(status);

    for (name, value) in headers {
        if let Some(name) = name {
            response = response.header(name, value);
        }
    }

    Ok(response.body(body.into())?)
}

#[async_trait]
impl<T: TokenManager + Send + Sync + Clone> FromRequestParts<Context<T>> for EdrClaims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        ctx: &Context<T>,
    ) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|err| {
                debug!("Error extracting the auth header {}", err);
                AuthError::InvalidToken
            })?;

        let token_data = ctx.tokens().validate(bearer.token()).map_err(|err| {
            debug!("Error decoding the bearer token {:#}", err);
            match err {
                crate::core::service::token::TokenError::Decode(e)
                    if e.kind() == &ErrorKind::ExpiredSignature =>
                {
                    AuthError::ExpiredToken
                }
                _ => AuthError::InvalidToken,
            }
        })?;

        Ok(token_data.claims)
    }
}

pub enum PublicError {
    Generic(anyhow::Error),
    TransferNotValid,
    ProxyError(reqwest::Error),
    AxumError(axum::http::Error),
}

impl IntoResponse for PublicError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            PublicError::TransferNotValid => {
                (StatusCode::FORBIDDEN, "Transfer not valid or not found")
            }
            PublicError::ProxyError(_e) => (StatusCode::BAD_GATEWAY, "Bad gateway"),
            PublicError::AxumError(_e) => (StatusCode::BAD_REQUEST, "Bad Request"),
            PublicError::Generic(_e) => (StatusCode::BAD_REQUEST, "Bad Request"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for PublicError {
    fn from(value: anyhow::Error) -> Self {
        PublicError::Generic(value)
    }
}

impl From<reqwest::Error> for PublicError {
    fn from(value: reqwest::Error) -> Self {
        PublicError::ProxyError(value)
    }
}

impl From<axum::http::Error> for PublicError {
    fn from(value: axum::http::Error) -> Self {
        PublicError::AxumError(value)
    }
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    ExpiredToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::ExpiredToken => (StatusCode::FORBIDDEN, "Expired token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
