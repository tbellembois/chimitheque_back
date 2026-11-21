use std::collections::HashMap;

use chimitheque_types::{requestfilter::RequestFilter, unittype::UnitType};
use http::HeaderMap;
use rusqlite::Connection;

use crate::{constants::CHIMITHEQUE_PERSON_ID_HEADER, errors::AppError};

/// Returns the type of every unit, by unit id, as stored in the database.
///
/// Used to validate unit fields: a request may carry only the unit id, so the
/// unit type must not be read from the request.
pub(crate) fn get_unit_types(
    db_connection: &Connection,
) -> Result<HashMap<u64, UnitType>, AppError> {
    let (units, _) = chimitheque_db::unit::get_units(db_connection, RequestFilter::default())
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(units
        .into_iter()
        .filter_map(|unit| unit.unit_id.map(|unit_id| (unit_id, unit.unit_type)))
        .collect())
}

pub(crate) fn get_chimitheque_person_id_from_headers(headers: &HeaderMap) -> Result<u64, AppError> {
    let Some(chimitheque_person_id_header) = headers.get(CHIMITHEQUE_PERSON_ID_HEADER) else {
        return Err(AppError::ChimithequePersonIdHeaderMissing);
    };

    let chimitheque_person_id_header_str = match chimitheque_person_id_header.to_str() {
        Ok(chimitheque_person_id_header_str) => chimitheque_person_id_header_str,
        Err(err) => return Err(AppError::ChimithequePersonIdHeaderInvalid(err.to_string())),
    };

    let chimitheque_person_id_u64: u64 = match chimitheque_person_id_header_str.parse() {
        Ok(chimitheque_person_id_u64) => chimitheque_person_id_u64,
        Err(err) => return Err(AppError::ChimithequePersonIdHeaderInvalid(err.to_string())),
    };

    Ok(chimitheque_person_id_u64)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn test_get_unit_types_from_base_data() {
        unsafe {
            std::env::set_var("SQLITE_EXTENSION_DIR", "src/extensions");
        };
        let mut db_connection = chimitheque_db::init::connect_test();
        chimitheque_db::init::create_tables(&mut db_connection).unwrap();
        chimitheque_db::init::populate_db_with_base_data(&mut db_connection).unwrap();

        let unit_types = get_unit_types(&db_connection).unwrap();

        // All base units, by id: L, °C, mg/L and g/mol.
        assert_eq!(unit_types.len(), 23);
        assert_eq!(unit_types.get(&1), Some(&UnitType::Quantity));
        assert_eq!(unit_types.get(&14), Some(&UnitType::Temperature));
        assert_eq!(unit_types.get(&21), Some(&UnitType::Concentration));
        assert_eq!(unit_types.get(&24), Some(&UnitType::MolecularWeight));
    }
}
