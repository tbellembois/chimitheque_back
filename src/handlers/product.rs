use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chimitheque_types::{product::Product, requestfilter::RequestFilter};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::{AppState, errors::AppError, utils::get_chimitheque_person_id_from_headers};

pub async fn get_products(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<Box<dyn erased_serde::Serialize>>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_products = chimitheque_db::product::get_products(
        db_connection.deref(),
        request_filter.clone(),
        chimitheque_person_id,
    );

    if request_filter.id.is_none() {
        match mayerr_products {
            Ok(products) => Ok(Json(Box::new(GetProductsOldResponse {
                rows: products.0,
                total: products.1,
            }))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    } else {
        match mayerr_products {
            Ok(products) => Ok(Json(Box::new(products.0.first().unwrap().to_owned()))),
            Err(err) => Err(AppError::Database(err.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GetProductsOldResponse {
    rows: Vec<Product>,
    total: usize,
}

pub async fn get_products_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<Json<GetProductsOldResponse>, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_products = chimitheque_db::product::get_products(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_products {
        Ok(products) => Ok(Json(GetProductsOldResponse {
            rows: products.0,
            total: products.1,
        })),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn create_update_product(
    State(state): State<AppState>,
    Json(product): Json<Product>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the product.
    let mut product = product.clone();
    if let Err(err) = product.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    let mayerr_product_id =
        chimitheque_db::product::create_update_product(db_connection.deref_mut(), product);

    match mayerr_product_id {
        Ok(product_id) => Ok(Json(product_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<(), AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let mut db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::product::delete_product(db_connection.deref_mut(), id) {
        Ok(_) => Ok(()),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn export_products(
    State(state): State<AppState>,
    headers: HeaderMap,
    request_filter: RequestFilter,
) -> Result<String, AppError> {
    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return Err(err),
    };

    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    let mayerr_products = chimitheque_db::product::export_products(
        db_connection.deref(),
        request_filter,
        chimitheque_person_id,
    );

    match mayerr_products {
        Ok(products) => Ok(products),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
