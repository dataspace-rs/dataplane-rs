use std::str;

use axum::http::{uri::InvalidUri, Uri};
use edc_dataplane_core::core::model::transfer::types::HttpData;
use edc_dataplane_core::core::model::transfer::{Transfer, TransferStatus};
use futures::TryFutureExt;
use pingora::http::RequestHeader;
use pingora::{upstreams::peer::HttpPeer, Result};
use pingora_proxy::{ProxyHttp, Session};
use tracing::debug;

use crate::model::edr::EdrEntry;
use crate::{
    web::state::Context,
    {
        model::edr::EdrClaims,
        service::token::{TokenError, TokenManager},
    },
};

const PUBLIC_PATH: &str = "/api/v1/public";

pub struct PublicProxy<T: TokenManager + Clone> {
    ctx: Context<T>,
}

impl<T: TokenManager + Clone> PublicProxy<T> {
    pub fn new(ctx: Context<T>) -> Self {
        Self { ctx }
    }
}

#[derive(Default)]
pub struct PublicCtx {
    transfer: Option<TransferRequest>,
}
impl PublicCtx {
    pub fn transfer(&self) -> Result<&TransferRequest> {
        self.transfer.as_ref().ok_or_else(|| {
            pingora::Error::new(pingora::ErrorType::Custom("Transfer not found in context"))
        })
    }
}

#[async_trait::async_trait]
impl<T: TokenManager + Send + Sync + Clone + 'static> ProxyHttp for PublicProxy<T> {
    type CTX = PublicCtx;
    fn new_ctx(&self) -> Self::CTX {
        PublicCtx::default()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        if !self.can_handle(session) {
            session.respond_error(404).await?;
            return Ok(true);
        }

        match self.parse_upstream_request(session).await {
            Ok(req) => self.handle_upstream_request(session, req, ctx).await,
            Err(err) => {
                debug!("Failed to handle proxy request error: {:#}", err);
                session.respond_error(err.to_response_code()).await?;
                Ok(true)
            }
        }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let host = ctx.transfer()?.upstream_host();
        let tls = ctx.transfer()?.is_tls();
        let port = ctx.transfer()?.upstream_port();

        Ok(Box::new(HttpPeer::new(
            (host.to_string(), port),
            tls,
            host.to_string(),
        )))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut pingora::http::RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request.remove_header("Authorization");
        upstream_request
            .insert_header("Host", ctx.transfer()?.upstream_host())
            .unwrap();
        Ok(())
    }
}

impl<T: TokenManager + Send + Sync + Clone + 'static> PublicProxy<T> {
    async fn parse_upstream_request(
        &self,
        session: &Session,
    ) -> std::result::Result<TransferRequest, ProxyError> {
        self.validate_token(session.req_header())
            .and_then(|claims| self.fetch_edr(claims))
            .and_then(|edr| self.fetch_transfer(edr))
            .and_then(|transfer| self.parse_transfer(transfer))
            .await
    }

    async fn handle_upstream_request(
        &self,
        session: &mut Session,
        req: TransferRequest,
        ctx: &mut PublicCtx,
    ) -> Result<bool> {
        let upstream_uri = req.to_upstream_uri(session);
        session.req_header_mut().set_uri(upstream_uri);
        ctx.transfer = Some(req);

        Ok(false)
    }

    async fn validate_token(
        &self,
        req: &RequestHeader,
    ) -> std::result::Result<EdrClaims, ProxyError> {
        req.headers
            .get("Authorization")
            .ok_or(ProxyError::MissingToken)
            .and_then(|token| str::from_utf8(token.as_bytes()).map_err(ProxyError::Utf8Error))
            .and_then(|mut token| {
                if token.starts_with("Bearer ") {
                    token = &token[7..];
                }

                self.ctx
                    .tokens()
                    .validate::<EdrClaims>(token)
                    .map_err(ProxyError::TokenError)
            })
            .map(|data| data.claims)
    }

    async fn fetch_transfer(&self, edr: EdrEntry) -> std::result::Result<Transfer, ProxyError> {
        self.ctx
            .transfers()
            .get(&edr.transfer_id)
            .await
            .map_err(ProxyError::Generic)?
            .filter(|transfer| transfer.status == TransferStatus::Started)
            .ok_or_else(|| ProxyError::InvalidTransfer)
    }

    async fn fetch_edr(&self, claims: EdrClaims) -> std::result::Result<EdrEntry, ProxyError> {
        self.ctx
            .edrs()
            .get_by_transfer_id(&claims.transfer_id)
            .await
            .map_err(ProxyError::Generic)?
            .filter(|edr| edr.token_id == claims.jti.into())
            .ok_or_else(|| ProxyError::InvalidTransfer)
    }

    async fn parse_transfer(
        &self,
        transfer: Transfer,
    ) -> std::result::Result<TransferRequest, ProxyError> {
        let data = HttpData::try_from(transfer.source.as_ref())?;

        Ok(TransferRequest { data })
    }

    fn can_handle(&self, session: &Session) -> bool {
        session.req_header().uri.path().starts_with(PUBLIC_PATH)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    #[error(transparent)]
    TokenError(TokenError),
    #[error("Missing Token")]
    MissingToken,
    #[error("Invalid Transfer")]
    InvalidTransfer,
    #[error(transparent)]
    Utf8Error(str::Utf8Error),
    #[error(transparent)]
    Generic(#[from] anyhow::Error),
    #[error(transparent)]
    InvalidUri(#[from] InvalidUri),
}

impl ProxyError {
    pub fn to_response_code(&self) -> u16 {
        match self {
            ProxyError::TokenError(_) => 403,
            ProxyError::MissingToken => 403,
            ProxyError::InvalidTransfer => 403,
            ProxyError::Utf8Error(_) => 502,
            ProxyError::Generic(_) => 502,
            ProxyError::InvalidUri(_) => 502,
        }
    }
}

pub struct TransferRequest {
    data: HttpData,
}

impl TransferRequest {
    pub fn upstream_host(&self) -> &str {
        self.data.base_url.host().unwrap()
    }

    pub fn is_tls(&self) -> bool {
        self.data
            .base_url
            .scheme()
            .map(|f| f.as_str() == "https")
            .unwrap_or_default()
    }

    pub fn upstream_port(&self) -> u16 {
        self.data
            .base_url
            .port_u16()
            .unwrap_or_else(|| if self.is_tls() { 443 } else { 80 })
    }

    pub fn to_upstream_uri(&self, session: &Session) -> Uri {
        let req_path = session.req_header().uri.path().replace(PUBLIC_PATH, "");

        Uri::builder()
            .path_and_query(&(self.data.base_url.path().to_string() + &req_path))
            .build()
            .unwrap()
    }
}
