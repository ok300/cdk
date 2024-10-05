//! Axum server for Mint

#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]

use std::sync::Arc;

use anyhow::Result;
use axum::routing::{get, post};
use axum::Router;
use cdk::error::{ErrorCode, ErrorResponse};
use cdk::amount::Amount;
use cdk::mint::Mint;
use cdk::nuts::nut00::{CurrencyUnit, PaymentMethod};
use cdk::nuts::nut01::{Keys, KeysResponse, PublicKey};
use cdk::nuts::nut02::{Id, KeySet, KeySetVersion};
use cdk::nuts::nut04;
use cdk::nuts::nut04::MintMethodSettings;
use cdk::nuts::nut05;
use cdk::nuts::nut05::MeltMethodSettings;
use cdk::nuts::nut06::{ContactInfo, MintInfo, MintVersion, Nuts, SupportedSettings};
use cdk::nuts::nut15;
use cdk::nuts::nut15::MppMethodSettings;
use router_handlers::*;
use utoipa::OpenApi;

mod router_handlers;

#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            Amount,
            ContactInfo,
            CurrencyUnit,
            ErrorCode,
            ErrorResponse,
            Id,
            Keys,
            KeysResponse,
            KeySet,
            KeySetVersion,
            MeltMethodSettings,
            MintInfo,
            MintMethodSettings,
            MintVersion,
            MppMethodSettings,
            Nuts,
            PaymentMethod,
            PublicKey,
            SupportedSettings,
            nut04::Settings,
            nut05::Settings,
            nut15::Settings
        )
    ),
    info(
        description = "Cashu CDK mint APIs",
        title = "cdk-mintd",
    ),
    paths(
        get_keys,
        get_keyset_pubkeys,
        get_mint_info
    )
)]
/// OpenAPI spec for the mint's v1 APIs
pub struct ApiDocV1;

/// Create mint [`Router`] with required endpoints for cashu mint
pub async fn create_mint_router(mint: Arc<Mint>) -> Result<Router> {
    let state = MintState { mint };

    let v1_router = Router::new()
        .route("/keys", get(get_keys))
        .route("/keysets", get(get_keysets))
        .route("/keys/:keyset_id", get(get_keyset_pubkeys))
        .route("/swap", post(post_swap))
        .route("/mint/quote/bolt11", post(get_mint_bolt11_quote))
        .route(
            "/mint/quote/bolt11/:quote_id",
            get(get_check_mint_bolt11_quote),
        )
        .route("/mint/bolt11", post(post_mint_bolt11))
        .route("/melt/quote/bolt11", post(get_melt_bolt11_quote))
        .route(
            "/melt/quote/bolt11/:quote_id",
            get(get_check_melt_bolt11_quote),
        )
        .route("/melt/bolt11", post(post_melt_bolt11))
        .route("/checkstate", post(post_check))
        .route("/info", get(get_mint_info))
        .route("/restore", post(post_restore));

    let mint_router = Router::new().nest("/v1", v1_router).with_state(state);

    Ok(mint_router)
}

/// CDK Mint State
#[derive(Clone)]
pub struct MintState {
    mint: Arc<Mint>,
}
