use axum::{Json, extract::Query};
use chimitheque_types::person::Person;
use chimitheque_utils::{
    casnumber::is_cas_number, cenumber::is_ce_number, formula::sort_empirical_formula,
};

use crate::errors::AppError;

pub async fn validate_email(Query(email): Query<String>) -> Result<Json<bool>, AppError> {
    let mut person = Person {
        person_email: email,
        ..Default::default()
    };

    match person.sanitize_and_validate() {
        Ok(_) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

pub async fn validate_cas_number(Query(cas_number): Query<String>) -> Result<Json<bool>, AppError> {
    match is_cas_number(cas_number.as_str()) {
        Ok(_) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

pub async fn validate_ce_number(Query(ce_number): Query<String>) -> Result<Json<bool>, AppError> {
    match is_ce_number(ce_number.as_str()) {
        Ok(_) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}

pub async fn validate_empirical_formula(
    Query(empirical_formula): Query<String>,
) -> Result<Json<bool>, AppError> {
    match sort_empirical_formula(empirical_formula.as_str()) {
        Ok(_) => Ok(Json(true)),
        Err(err) => Err(AppError::InputValidation(err.to_string())),
    }
}
