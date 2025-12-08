use chimitheque_back::run;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let app_url = std::env::var("APP_URL").expect("APP_URL env variable");
    let db_path = std::env::var("DB_PATH").expect("DB_PATH env variable");
    let issuer = std::env::var("ISSUER").expect("ISSUER env variable");
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID env variable");
    let client_secret = std::env::var("CLIENT_SECRET").ok();
    run(app_url, db_path, issuer, client_id, client_secret).await
}
