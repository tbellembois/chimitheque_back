use axum::{Json, extract::State};
use axum_extra::extract::Query;
use chimitheque_db::searchable::get_many;
use chimitheque_traits::searchable::Searchable;
use chimitheque_types::{
    casnumber::CasNumber, category::Category, cenumber::CeNumber, classofcompound::ClassOfCompound,
    empiricalformula::EmpiricalFormula, hazardstatement::HazardStatement,
    linearformula::LinearFormula, name::Name, physicalstate::PhysicalState,
    precautionarystatement::PrecautionaryStatement, producer::Producer, producerref::ProducerRef,
    requestfilter::RequestFilter, signalword::SignalWord, supplier::Supplier,
    supplierref::SupplierRef, symbol::Symbol, tag::Tag,
};
use chimitheque_utils::string::Transform;
use serde::Serialize;
use std::ops::Deref;

use crate::{appstate::AppState, errors::AppError};

pub async fn get_cas_numbers(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &CasNumber {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((cas_numbers, count)) => Ok(Json((cas_numbers, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_ce_numbers(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &CeNumber {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((ce_numbers, count)) => Ok(Json((ce_numbers, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_categories(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Category {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((categories, count)) => Ok(Json((categories, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_classes_of_compounds(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &ClassOfCompound {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((classes_of_compounds, count)) => Ok(Json((classes_of_compounds, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_empirical_formulas(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &EmpiricalFormula {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((empirical_formulas, count)) => Ok(Json((empirical_formulas, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_linear_formulas(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &LinearFormula {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((linear_formulas, count)) => Ok(Json((linear_formulas, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_names(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Name {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((names, count)) => Ok(Json((names, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_physical_states(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &PhysicalState {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((physical_states, count)) => Ok(Json((physical_states, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_symbols(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Symbol {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((symbols, count)) => Ok(Json((symbols, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_tags(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Tag {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((tags, count)) => Ok(Json((tags, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_signal_words(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &SignalWord {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((signal_words, count)) => Ok(Json((signal_words, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_hazard_statements(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &HazardStatement {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((hazard_statements, count)) => Ok(Json((hazard_statements, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_precautionary_statements(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &PrecautionaryStatement {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((precautionary_statements, count)) => Ok(Json((precautionary_statements, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_producers(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Producer {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((producers, count)) => Ok(Json((producers, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_suppliers(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<impl Searchable + Serialize>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match get_many(
        &Supplier {
            ..Default::default()
        },
        db_connection.deref(),
        request_filter,
    ) {
        Ok((suppliers, count)) => Ok(Json((suppliers, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_producer_refs(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<ProducerRef>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::producerref::get_producer_refs(db_connection.deref(), request_filter) {
        Ok((producer_refs, count)) => Ok(Json((producer_refs, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn get_supplier_refs(
    State(state): State<AppState>,
    Query(request_filter): Query<RequestFilter>,
) -> Result<Json<(Vec<SupplierRef>, usize)>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    match chimitheque_db::supplierref::get_supplier_refs(db_connection.deref(), request_filter) {
        Ok((supplier_refs, count)) => Ok(Json((supplier_refs, count))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn create_producer(
    State(state): State<AppState>,
    Json(producer): Json<Producer>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the producer.
    let mut producer = producer.clone();
    if let Err(err) = producer.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    match chimitheque_db::searchable::create_update(
        &Producer {
            ..Default::default()
        },
        None,
        &db_connection,
        &producer.producer_label,
        Transform::None,
    ) {
        Ok(producer_id) => Ok(Json(producer_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

pub async fn create_supplier(
    State(state): State<AppState>,
    Json(supplier): Json<Supplier>,
) -> Result<Json<u64>, AppError> {
    // Get the connection from the database.
    let db_connection_pool = state.db_connection_pool.clone();
    let db_connection = db_connection_pool.get().unwrap();

    // Sanitize and validate the supplier.
    let mut supplier = supplier.clone();
    if let Err(err) = supplier.sanitize_and_validate() {
        return Err(AppError::InputValidation(err.to_string()));
    };

    match chimitheque_db::searchable::create_update(
        &Supplier {
            ..Default::default()
        },
        None,
        &db_connection,
        &supplier.supplier_label,
        Transform::None,
    ) {
        Ok(supplier_id) => Ok(Json(supplier_id)),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}
