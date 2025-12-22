use axum::Json;

pub async fn fake() -> Json<bool> {
    Json(true)
}
