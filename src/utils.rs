use http::HeaderMap;

use crate::errors::AppError;

pub(crate) fn get_chimitheque_person_id_from_headers(headers: &HeaderMap) -> Result<u64, AppError> {
    let Some(chimitheque_person_id_header) = headers.get("chimitheque_person_id") else {
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
