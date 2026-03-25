use axum::{
    Json,
    extract::{Path, Query},
};
use chimitheque_utils::{
    casnumber::is_cas_number, cenumber::is_ce_number, formula::to_empirical_formula,
};
use serde::Deserialize;

use crate::errors::AppError;

#[derive(Deserialize)]
pub struct CasQuery {
    cas_number: String,
}

pub async fn validate_cas_number_old(
    Query(params): Query<CasQuery>,
) -> Result<Json<bool>, AppError> {
    match is_cas_number(params.cas_number.trim()) {
        Ok(()) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

#[derive(Deserialize)]
pub struct CeQuery {
    ce_number: String,
}

pub async fn validate_ce_number_old(Query(params): Query<CeQuery>) -> Result<Json<bool>, AppError> {
    match is_ce_number(params.ce_number.trim()) {
        Ok(()) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

#[derive(Deserialize)]
pub struct EmpiricalFormulaQuery {
    empirical_formula: String,
}

pub async fn validate_empirical_formula_old(
    Query(params): Query<EmpiricalFormulaQuery>,
) -> Result<Json<bool>, AppError> {
    match to_empirical_formula(params.empirical_formula.trim()) {
        Ok(_) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

#[derive(Deserialize)]
pub struct LinearToEmpiricalFormulaPathParameters {
    linear_formula: String,
}

pub async fn linear_to_empirical_formula(
    Path(path_params): Path<LinearToEmpiricalFormulaPathParameters>,
) -> Result<Json<String>, AppError> {
    match to_empirical_formula(path_params.linear_formula.trim()) {
        Ok(empirical_formula) => Ok(Json(empirical_formula)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}
