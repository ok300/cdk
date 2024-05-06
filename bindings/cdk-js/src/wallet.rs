use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use cdk::nuts::SigningKey;
use cdk::url::UncheckedUrl;
use cdk::wallet::Wallet;
use cdk::{Amount, HttpClient};
use cdk_rexie::RexieWalletDatabase;
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};
use crate::nuts::{JsCurrencyUnit, JsMintInfo};
use crate::types::melt_quote::JsMeltQuote;
use crate::types::{JsAmount, JsMelted, JsMintQuote};

#[wasm_bindgen(js_name = Wallet)]
pub struct JsWallet {
    inner: Wallet,
}

impl Deref for JsWallet {
    type Target = Wallet;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Wallet> for JsWallet {
    fn from(inner: Wallet) -> JsWallet {
        JsWallet { inner }
    }
}

#[wasm_bindgen(js_class = Wallet)]
impl JsWallet {
    #[wasm_bindgen(constructor)]
    pub async fn new() -> Self {
        let client = HttpClient::new();
        let db = RexieWalletDatabase::new().await.unwrap();

        Wallet::new(client, Arc::new(db), None).await.into()
    }

    #[wasm_bindgen(js_name = mintBalances)]
    pub async fn mint_balances(&self) -> Result<JsValue> {
        let mint_balances = self.inner.mint_balances().await.map_err(into_err)?;

        Ok(serde_wasm_bindgen::to_value(&mint_balances)?)
    }

    #[wasm_bindgen(js_name = addMint)]
    pub async fn add_mint(&self, mint_url: String) -> Result<Option<JsMintInfo>> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;

        Ok(self
            .inner
            .add_mint(mint_url)
            .await
            .map_err(into_err)?
            .map(|i| i.into()))
    }

    #[wasm_bindgen(js_name = refreshMint)]
    pub async fn refresh_mint_keys(&self, mint_url: String) -> Result<()> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;
        self.inner
            .refresh_mint_keys(&mint_url)
            .await
            .map_err(into_err)?;
        Ok(())
    }

    #[wasm_bindgen(js_name = mintQuote)]
    pub async fn mint_quote(
        &mut self,
        mint_url: String,
        amount: u64,
        unit: JsCurrencyUnit,
    ) -> Result<JsMintQuote> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;
        let quote = self
            .inner
            .mint_quote(mint_url, amount.into(), unit.into())
            .await
            .map_err(into_err)?;

        Ok(quote.into())
    }

    #[wasm_bindgen(js_name = mint)]
    pub async fn mint(&mut self, mint_url: String, quote_id: String) -> Result<JsAmount> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;

        Ok(self
            .inner
            .mint(mint_url, &quote_id)
            .await
            .map_err(into_err)?
            .into())
    }

    #[wasm_bindgen(js_name = meltQuote)]
    pub async fn melt_quote(
        &mut self,
        mint_url: String,
        unit: JsCurrencyUnit,
        request: String,
    ) -> Result<JsMeltQuote> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;
        let melt_quote = self
            .inner
            .melt_quote(mint_url, unit.into(), request)
            .await
            .map_err(into_err)?;

        Ok(melt_quote.into())
    }

    #[wasm_bindgen(js_name = melt)]
    pub async fn melt(&mut self, mint_url: String, quote_id: String) -> Result<JsMelted> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;

        let melted = self
            .inner
            .melt(&mint_url, &quote_id)
            .await
            .map_err(into_err)?;

        Ok(melted.into())
    }

    #[wasm_bindgen(js_name = receive)]
    pub async fn receive(
        &mut self,
        encoded_token: String,
        signing_keys: JsValue,
        preimages: JsValue,
    ) -> Result<()> {
        let signing_keys: Option<Vec<SigningKey>> = serde_wasm_bindgen::from_value(signing_keys)?;
        let preimages: Option<Vec<String>> = serde_wasm_bindgen::from_value(preimages)?;

        self.inner
            .receive(&encoded_token, signing_keys, preimages)
            .await
            .map_err(into_err)?;

        Ok(())
    }

    #[wasm_bindgen(js_name = send)]
    pub async fn send(
        &mut self,
        mint_url: String,
        unit: JsCurrencyUnit,
        memo: Option<String>,
        amount: u64,
    ) -> Result<String> {
        let mint_url = UncheckedUrl::from_str(&mint_url).map_err(into_err)?;

        self.inner
            .send(&mint_url, &unit.into(), memo, Amount::from(amount), None)
            .await
            .map_err(into_err)
    }
}
