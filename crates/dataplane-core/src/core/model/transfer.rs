use bon::Builder;
use sqlx::{prelude::FromRow, types::Json};

use crate::signaling::DataAddress;

use super::edr::{RefreshTokenId, TokenId};

#[derive(Builder, Clone, Debug, FromRow, PartialEq)]
pub struct Transfer {
    pub id: String,
    pub status: TransferStatus,
    #[builder(into)]
    pub source: Json<DataAddress>,
    pub token_id: TokenId,
    pub refresh_token_id: RefreshTokenId,
    #[builder(default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[builder(default)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, sqlx::Type, PartialEq)]
pub enum TransferStatus {
    Started,
    Suspended,
}

pub mod types {
    use axum::http::Uri;

    use crate::{core::model::namespace::EDC_NAMESPACE, signaling::DataAddress};

    pub enum TransferKind {
        HttpData(HttpData),
    }

    impl TryFrom<&DataAddress> for TransferKind {
        type Error = anyhow::Error;

        fn try_from(value: &DataAddress) -> Result<Self, Self::Error> {
            match value.endpoint_type.as_str() {
                "HttpData" => Ok(TransferKind::HttpData(HttpData::try_from(value)?)),
                kind if kind == EDC_NAMESPACE.to_iri("HttpData") => {
                    Ok(TransferKind::HttpData(HttpData::try_from(value)?))
                }
                _ => Err(anyhow::anyhow!("Unsupported endpoint type")),
            }
        }
    }

    pub struct HttpData {
        pub base_url: Uri,
        pub proxy_path: bool,
        pub proxy_method: bool,
        pub proxy_query_params: bool,
    }

    impl TryFrom<&DataAddress> for HttpData {
        type Error = anyhow::Error;

        fn try_from(value: &DataAddress) -> Result<Self, Self::Error> {
            Ok(Self {
                base_url: value
                    .get_property(&EDC_NAMESPACE.to_iri("baseUrl"))
                    .map(|url| url.parse::<Uri>())
                    .ok_or_else(|| anyhow::anyhow!("Missing base url"))??,
                proxy_path: get_bool_property(value, "proxyPath"),
                proxy_method: get_bool_property(value, "proxyMethod"),
                proxy_query_params: get_bool_property(value, "proxyQueryParams"),
            })
        }
    }

    fn get_bool_property(value: &DataAddress, property: &str) -> bool {
        value
            .get_property(&EDC_NAMESPACE.to_iri(property))
            .map(|v| v.parse::<bool>())
            .unwrap_or_else(|| Ok(false))
            .unwrap_or(false)
    }
}
