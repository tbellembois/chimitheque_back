use std::ops::Deref;

use axum::{
    Json,
    extract::{Query, State},
};
use chimitheque_pubchem::{pubchem::get_product_by_name, pubchem_compound::Autocomplete};
use chimitheque_pubchem::{
    pubchem::{autocomplete, get_compound_by_name},
    pubchem_compound::Record,
};
use chimitheque_types::pubchemproduct::PubchemProduct;

use crate::{appstate::AppState, errors::AppError};

pub async fn pubchem_autocomplete(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Autocomplete>, AppError> {
    let rate_limiter = state.rate_limiter;

    match autocomplete(rate_limiter.deref(), name.as_str()) {
        Ok(autocomplete) => Ok(Json(autocomplete)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}

pub async fn pubchem_getcompoundbyname(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Record>, AppError> {
    let rate_limiter = state.rate_limiter;

    match get_compound_by_name(rate_limiter.deref(), name.as_str()) {
        Ok(record) => Ok(Json(record)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}

pub async fn pubchem_getproductbyname(
    State(state): State<AppState>,
    Query(name): Query<String>,
) -> Result<Json<Option<PubchemProduct>>, AppError> {
    let rate_limiter = state.rate_limiter;

    match get_product_by_name(rate_limiter.deref(), name.as_str()) {
        Ok(maybe_pubchemproduct) => Ok(Json(maybe_pubchemproduct)),
        Err(err) => Err(AppError::Pubchem(err.to_string())),
    }
}
