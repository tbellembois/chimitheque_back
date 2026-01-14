use chimitheque_back::run;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_path = std::env::var("DB_PATH").expect("DB_PATH env variable");
    let admins = std::env::var("ADMINS").unwrap_or_default();
    let keycloak_base_url =
        std::env::var("KEYCLOAK_BASE_URL").expect("KEYCLOAK_BASE_URL env variable");
    let keycloak_realm = std::env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM env variable");
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID env variable");
    run(
        db_path,
        admins,
        keycloak_base_url,
        keycloak_realm,
        client_id,
    )
    .await
}
