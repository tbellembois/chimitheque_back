pub mod appstate;
pub mod errors;
pub mod handlers;
pub mod utils;

use crate::{
    appstate::AppState,
    errors::AppError,
    handlers::{
        bookmark::toogle_bookmark,
        borrowing::toogle_borrowing,
        entity::{
            create_update_entity, delete_entity, get_entities, get_entities_old, get_entity_stock,
        },
        fake::fake,
        person::{
            create_update_person, delete_person, get_connected_user, get_people, get_people_old,
        },
        product::{
            create_update_product, delete_product, export_products, get_products, get_products_old,
        },
        pubchem::{
            pubchem_autocomplete, pubchem_create_update_product, pubchem_getcompoundbyname,
            pubchem_getproductbyname,
        },
        searchable::{
            create_producer, create_supplier, get_cas_numbers, get_cas_numbers_old, get_categories,
            get_categories_old, get_ce_numbers, get_ce_numbers_old, get_classes_of_compounds,
            get_classes_of_compounds_old, get_empirical_formulas, get_empirical_formulas_old,
            get_hazard_statements, get_hazard_statements_old, get_linear_formulas,
            get_linear_formulas_old, get_names, get_names_old, get_physical_states,
            get_physical_states_old, get_precautionary_statements,
            get_precautionary_statements_old, get_producer_refs, get_producer_refs_old,
            get_producers, get_producers_old, get_signal_words, get_signal_words_old,
            get_supplier_refs, get_supplier_refs_old, get_suppliers, get_suppliers_old,
            get_symbols, get_symbols_old, get_tags, get_tags_old, get_units, get_units_old,
        },
        storage::{
            archive_storage, create_update_storage, delete_storage, export_storages, get_storages,
            get_storages_old, unarchive_storage,
        },
        store_location::{
            create_update_store_location, delete_store_location, get_store_locations,
            get_store_locations_old,
        },
        validate::{
            validate_cas_number, validate_ce_number, validate_email, validate_empirical_formula,
        },
    },
    utils::get_chimitheque_person_id_from_headers,
};

use axum::{
    Extension, Router,
    extract::{Request, State},
    http::HeaderMap,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use casbin::{CoreApi, DefaultModel, Enforcer, StringAdapter};
use chimitheque_db::{
    casbin::to_string_adapter,
    init::{init_db, update_ghs_statements},
};
use chimitheque_types::requestfilter::RequestFilter;
use chrono::Local;
use dashmap::DashMap;
use governor::{Quota, RateLimiter};
use http::Method;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use r2d2::{self};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use serde::Deserialize;
use std::{
    env,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};
use std::{io::Write, os::unix::fs::MetadataExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};
use ureq::config::Config;
use url::Url;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub sub: String,   // claims sub
    pub email: String, // claims email
}

#[derive(Debug, Deserialize)]
struct Claims {
    email: Option<String>,
    sub: String, // Keycloak user ID (UUID)
                 // iss: String, // The iss (issuer) claim identifies the principal that issued the JWT.

                 // Keycloak: string OR array
                 // #[serde(default)]
                 // aud: Option<serde_json::Value>, // A string or array of strings that identifies the recipients that the JWT is intended for.
}

static JWKS_CACHE: OnceCell<Mutex<JwksCache>> = OnceCell::new();

#[derive(Clone)]
pub struct AccessToken(pub String);

#[derive(Debug, Deserialize)]
struct RsaJwk {
    n: String,
    e: String,
    kid: String,
    // kty: String,
    // r#use: String,
    // alg: String,
}

// Cached JWKS with timestamp.
#[derive(Debug, Deserialize)]
struct JwksCache {
    keys: Vec<RsaJwk>,

    #[serde(skip_deserializing)]
    #[serde(default = "std::time::Instant::now")]
    last_updated: std::time::Instant,
}

// Refresh JWKS from Keycloak
fn refresh_jwks(
    http_client: &Arc<ureq::Agent>,
    keycloak_base_url: String,
) -> Result<JwksCache, AppError> {
    let url = format!(
        "{}/realms/chimitheque/protocol/openid-connect/certs",
        keycloak_base_url
    );

    match http_client.get(url).call() {
        Ok(mut response) => match response.body_mut().read_json::<JwksCache>() {
            Ok(jwks) => Ok(jwks),
            Err(err) => Err(AppError::DecodeJWKS(err.to_string())),
        },
        Err(err) => Err(AppError::CertificatesRetrieval(err.to_string())),
    }
}

pub async fn jwt_middleware(
    State(state): State<AppState>,
    Extension(http_client): Extension<Arc<ureq::Agent>>,
    mut req: Request,
    next: Next,
) -> Response {
    debug!("jwt_middleware");

    // Extract Bearer token.
    let token = match req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        Some(token) => token,
        None => {
            return AppError::BearerTokenMissing.into_response();
        }
    };

    // Decode header to get kid.
    let header = match decode_header(token) {
        Ok(header) => header,
        Err(err) => return AppError::DecodeJWTHeader(err.to_string()).into_response(),
    };

    // Extract kid from header.
    // The kid (key ID) Header Parameter is a hint indicating which key was used to secure the JWS.
    let kid = match header.kid {
        Some(kid) => kid,
        None => return AppError::HeaderKIDMissing.into_response(),
    };

    // Get JWKS cache
    // The JSON Web Key Set (JWKS) is a set of keys containing the public keys used to verify any issued by the and signed using the RS256 signing algorithm.
    let cache = JWKS_CACHE.get_or_init(|| {
        let keys = match refresh_jwks(&http_client, state.keycloak_base_url.clone()) {
            Ok(jwks_cache) => jwks_cache.keys,
            Err(err) => {
                error!("{}", AppError::RefreshJWKS(err.to_string()));
                vec![]
            }
        };

        Mutex::new(JwksCache {
            keys,
            last_updated: std::time::Instant::now(),
        })
    });

    let mut jwks_lock = cache.lock().await;

    // Refresh JWKS if older than 10 minutes or kid not found.
    let kid_found = jwks_lock.keys.iter().any(|k| k.kid == kid);
    if !kid_found || jwks_lock.last_updated.elapsed() > std::time::Duration::from_secs(600) {
        let keys = match refresh_jwks(&http_client, state.keycloak_base_url.clone()) {
            Ok(jwks_cache) => jwks_cache.keys,
            Err(err) => return AppError::RefreshJWKS(err.to_string()).into_response(),
        };

        jwks_lock.keys = keys;
        jwks_lock.last_updated = std::time::Instant::now();
    }

    // Find key by kid.
    let rsa_jwk = match jwks_lock.keys.iter().find(|k| k.kid == kid) {
        Some(jwk) => jwk,
        None => return AppError::RSAJWKNotFoundInCache(kid).into_response(),
    };

    // Decode and validate claims, check expected audience.
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[format!("{}/realms/chimitheque", state.keycloak_base_url)]);
    validation.set_audience(&["chimitheque"]);

    let claims: Claims = match decode::<Claims>(
        token,
        &DecodingKey::from_rsa_components(&rsa_jwk.n, &rsa_jwk.e).unwrap(),
        &validation,
    ) {
        Ok(token_data) => token_data.claims,
        Err(err) => {
            return AppError::ClaimsDecoding(err.to_string()).into_response();
        }
    };

    // Inject username (sub) into request extensions.
    let user_email = claims.email.unwrap();
    let auth_context = AuthContext {
        sub: claims.sub,
        email: user_email,
    };
    req.extensions_mut().insert(auth_context);

    // ✅ Continue to next middleware/handler
    next.run(req).await
}

// Extract OIDC claims from the request.
// Insert the authenticated user id and email into the request headers.
async fn authenticate_middleware(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    mut request: Request,
    next: Next,
) -> Response {
    debug!("authenticate_middleware");

    if request.uri().path() == "/login" {
        return next.run(request).await;
    }

    let db_connection = state.db_connection_pool.get().unwrap();

    if auth.email.is_empty() {
        return AppError::MissingEmailInClaims.into_response();
    };

    // Get the person from the database.
    let (people, _) = match chimitheque_db::person::get_people(
        db_connection.deref(),
        RequestFilter {
            person_email: Some(auth.email),
            ..Default::default()
        },
        1,
    ) {
        Ok(people) => people,
        Err(err) => return AppError::Database(err.to_string()).into_response(),
    };

    // We trust the result from get_people and then expect one result that is not None.
    let person = people.first().unwrap();

    request.headers_mut().insert(
        "chimitheque_person_id",
        person.person_id.unwrap().to_string().parse().unwrap(),
    );
    request.headers_mut().insert(
        "chimitheque_person_email",
        person.person_email.parse().unwrap(),
    );

    next.run(request).await
}

// Authorize the connected user to perform the request action.
// Use the casbin enforcer (in the state object) to check the user's permissions.
async fn authorize_middleware(
    State(state): State<AppState>,
    id: Option<axum::extract::path::Path<String>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    debug!("authorize_middleware");

    if request.uri().path() == "/login" {
        return next.run(request).await;
    }

    // Get the item ID as a string.
    let item_id = match id {
        Some(id) => id.to_string(),
        None => String::new(),
    };

    // Get the chimitheque_person_id.
    let chimitheque_person_id = match get_chimitheque_person_id_from_headers(&headers) {
        Ok(chimitheque_person_id) => chimitheque_person_id,
        Err(err) => return err.into_response(),
    };

    // Get the request method.
    let request_method = request.method();

    // Method <-> CRUD action match.
    let request_action = match *request_method {
        Method::GET => String::from("r"),
        Method::POST => String::from("c"),
        Method::PUT => String::from("u"),
        Method::DELETE => String::from("d"),
        _ => String::from("unknown"),
    };

    // Get the request URI.
    let request_uri = request.uri();
    // Adding a fake host required by the parse function below.
    let request_full = format!(
        "http://localhost{}",
        request_uri.path().trim_start_matches("/f")
    );

    debug!("request_full: {}", request_full);

    // Parse the request URI.
    let mayerr_parsed_uri = Url::parse(&request_full);
    if let Err(err) = mayerr_parsed_uri {
        return AppError::ParseURI(err.to_string()).into_response();
    }

    // Get the first path segment.
    let maybe_first_segment = mayerr_parsed_uri
        .unwrap()
        .path_segments()
        .and_then(|mut segments| segments.next().map(|segment| segment.to_string()));

    // Then the casbin item.
    let Some(item) = maybe_first_segment else {
        return AppError::InvalidFirstPathSegment(maybe_first_segment).into_response();
    };

    debug!("chimitheque_person_id: {}", chimitheque_person_id);
    debug!("request_action: {}", request_action);
    debug!("item: {}", item);
    debug!("item_id: {}", item_id);

    // Check the protected endpoints.
    if [
        String::from("store_locations"),
        String::from("people"),
        String::from("entities"),
        String::from("stocks"),
        String::from("products"),
        String::from("storages"),
        String::from("pubchemautocomplete"),
        String::from("pubchemgetcompoundbyname"),
        String::from("pubchemgetproductbyname"),
        String::from("pubchemproduct"),
        String::from("bookmarks"),
        String::from("borrows"),
    ]
    .contains(&item)
    {
        // Get the casbin enforcer from the state object.
        // TODO: https://github.com/tokio-rs/axum/discussions/2458
        let casbin_enforcer = state.casbin_enforcer.lock().await;

        match casbin_enforcer.enforce((
            chimitheque_person_id.to_string(),
            request_action,
            item,
            item_id,
        )) {
            Ok(true) => {
                debug!("authorize_middleware: true");
            }
            Ok(false) => {
                debug!("authorize_middleware: false");

                return AppError::PermissionDenied.into_response();
            }
            Err(err) => return AppError::CasbinError(err.to_string()).into_response(),
        };
    }

    next.run(request).await
}

// A debug middleware.
async fn _debug_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // do something with `request`...
    dbg!("REQUEST");

    let request_headers = request.headers();
    request_headers.iter().for_each(|header| {
        println!("request header: {:?} = {:?}", header.0, header.1);
    });

    let response = next.run(request).await;

    // do something with `response`...
    dbg!("RESPONSE");

    let response_headers = response.headers();
    response_headers.iter().for_each(|header| {
        println!("response header: {:?} = {:?}", header.0, header.1);
    });

    response
}

pub async fn run(
    db_path: String,
    keycloak_base_url: String,
    keycloak_redirect_url: String,
    keycloak_realm: String,
    client_id: String,
) {
    // Initialize logger.
    env_logger::builder()
        .format_timestamp_millis()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    // Create DB pool.
    info!("creating DB pool");

    let manager =
        SqliteConnectionManager::file(db_path.clone()).with_init(|conn: &mut Connection| {
            // Load SQLite extensions directory.
            let sql_extension_dir = env::var("SQLITE_EXTENSION_DIR")
                .expect("Missing SQLITE_EXTENSION_DIR environment variable.");

            // Enable extension loading
            unsafe { conn.load_extension_enable() }?;

            // Load the extension (example path)
            unsafe { conn.load_extension(format!("{}/{}", sql_extension_dir, "regexp.so"), None) }?;

            // Disable again for safety
            conn.load_extension_disable()?;

            Ok(())
        });

    let db_connection_pool = r2d2::Pool::builder().build(manager).unwrap();

    // Load extensions.
    let mut db_connection = db_connection_pool.get().unwrap();

    // Check that DB file exist, create if not.
    if Path::new(&db_path).metadata().unwrap().size() == 0 {
        info!("initialize DB");

        init_db(db_connection.deref_mut()).unwrap();
    } else {
        // Updating statements on already existing DB - panic on failure.
        let db_transaction = db_connection.transaction().unwrap();

        info!("updating GHS statements");

        update_ghs_statements(&db_transaction).unwrap();

        db_transaction.commit().unwrap();
    }

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(120)));

    // Initialize the Casbin toolkit.
    info!("initialize casbin");

    let casbin_string_adapter = to_string_adapter(db_connection.deref()).unwrap();
    let casbin_model = DefaultModel::from_file("casbin/policy.conf").await.unwrap();
    let casbin_adapter = StringAdapter::new(casbin_string_adapter.clone());
    let casbin_enforcer = Arc::new(Mutex::new(
        Enforcer::new(casbin_model, casbin_adapter).await.unwrap(),
    ));

    // Initialize rate limiter for pubchem requests.
    info!("initialize the request rate limiter");

    let rate_limiter = RateLimiter::direct(Quota::per_second(NonZeroU32::new(5).unwrap()));

    let mut state = AppState {
        casbin_enforcer,
        db_connection_pool: Arc::new(db_connection_pool),
        rate_limiter: Arc::new(rate_limiter),
        keycloak_client_id: client_id,
        keycloak_redirect_url,
        keycloak_realm,
        keycloak_base_url,
        pkce_store: Arc::new(Mutex::new(DashMap::new())),
    };

    info!("loading casbin enforcer functions");

    state.set_enforcer().await;

    //     requests
    //        |
    //        v
    // +----- layer_three -----+
    // | +---- layer_two ----+ |
    // | | +-- layer_one --+ | |
    // | | |               | | |
    // | | |    handler    | | |
    // | | |               | | |
    // | | +-- layer_one --+ | |
    // | +---- layer_two ----+ |
    // +----- layer_three -----+
    //        |
    //        v
    //     responses

    // Build your custom TLS HTTP client (with self-signed cert).
    let tls_config = ureq::tls::TlsConfig::builder()
        .disable_verification(true) // ✨ allow self-signed
        .build();

    // Build request config
    let config = Config::builder().tls_config(tls_config).build();

    // Create shared agent
    let http_client = Arc::new(config.new_agent());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    info!("initialize routes");

    let app = Router::new()
        //
        .route("/getconnecteduser", get(get_connected_user))
        //
        .route("/store_locations", get(get_store_locations))
        .route("/store_locations/{id}", get(get_store_locations))
        .route("/store_locations_old", get(get_store_locations_old))
        .route("/store_locations_old/{id}", get(get_store_locations_old))
        .route("/store_locations/{id}", put(create_update_store_location))
        .route("/store_locations", post(create_update_store_location))
        .route("/store_locations/{id}", delete(delete_store_location))
        //
        .route("/f/store_locations", get(fake))
        .route("/f/store_locations/{id}", get(fake))
        .route("/f/store_locations/{id}", put(fake))
        .route("/f/store_locations", post(fake))
        .route("/f/store_locations/{id}", delete(fake))
        //
        .route("/people", get(get_people))
        .route("/people/{id}", get(get_people))
        .route("/people_old", get(get_people_old))
        .route("/people_old/{id}", get(get_people_old))
        .route("/people/{id}", put(create_update_person))
        .route("/people", post(create_update_person))
        .route("/people/{id}", delete(delete_person))
        //
        .route("/f/people", get(fake))
        .route("/f/people/{id}", get(fake))
        .route("/f/people/{id}", put(fake))
        .route("/f/people", post(fake))
        .route("/f/people/{id}", delete(fake))
        //
        .route("/entities", get(get_entities))
        .route("/entities/{id}", get(get_entities))
        .route("/entities_old", get(get_entities_old))
        .route("/entities_old/{id}", get(get_entities_old))
        .route("/entities/{id}", put(create_update_entity))
        .route("/entities", post(create_update_entity))
        .route("/entities/{id}", delete(delete_entity))
        //
        .route("/f/entities", get(fake))
        .route("/f/entities/{id}", get(fake))
        .route("/f/entities/{id}", put(fake))
        .route("/f/entities", post(fake))
        .route("/f/entities/{id}", delete(fake))
        //
        .route("/stocks/{id}", get(get_entity_stock))
        //
        .route("/products", get(get_products))
        .route("/products/{id}", get(get_products))
        .route("/products_old", get(get_products_old))
        .route("/products_old/{id}", get(get_products_old))
        .route("/products/{id}", put(create_update_product))
        .route("/products", post(create_update_product))
        .route("/products/{id}", delete(delete_product))
        .route("/products/export", get(export_products))
        //
        .route("/f/products", get(fake))
        .route("/f/products/{id}", get(fake))
        .route("/f/products/{id}", put(fake))
        .route("/f/products", post(fake))
        .route("/f/products/{id}", delete(fake))
        //
        .route("/storages", get(get_storages))
        .route("/storages/{id}", get(get_storages))
        .route("/storages_old", get(get_storages_old))
        .route("/storages_old/{id}", get(get_storages_old))
        .route("/storages/{id}", put(create_update_storage))
        .route("/storages", post(create_update_storage))
        .route("/storages/{id}", delete(delete_storage))
        .route("/storages/export", get(export_storages))
        .route("/storages/{id}/archive", delete(archive_storage))
        .route("/storages/{id}/unarchive", put(unarchive_storage))
        //
        .route("/f/storages", get(fake))
        .route("/f/storages/{id}", get(fake))
        .route("/f/storages/{id}", put(fake))
        .route("/f/storages", post(fake))
        .route("/f/storages/{id}", delete(fake))
        //
        .route("/pubchemautocomplete/{name}", get(pubchem_autocomplete))
        .route(
            "/pubchemgetcompoundbyname/{name}",
            get(pubchem_getcompoundbyname),
        )
        .route(
            "/pubchemgetproductbyname/{name}",
            get(pubchem_getproductbyname),
        )
        .route("/pubchemproduct", post(pubchem_create_update_product))
        .route("/pubchemproduct/{id}", post(pubchem_create_update_product))
        //
        .route("/storages/units", get(get_units))
        .route("/storages/units_old", get(get_units_old))
        .route("/products/casnumbers", get(get_cas_numbers))
        .route("/products/casnumbers_old", get(get_cas_numbers_old))
        .route("/products/cenumbers", get(get_ce_numbers))
        .route("/products/cenumbers_old", get(get_ce_numbers_old))
        .route("/products/names", get(get_names))
        .route("/products/names_old", get(get_names_old))
        .route("/products/linearformulas", get(get_linear_formulas))
        .route("/products/linearformulas_old", get(get_linear_formulas_old))
        .route("/products/empiricalformulas", get(get_empirical_formulas))
        .route(
            "/products/empiricalformulas_old",
            get(get_empirical_formulas_old),
        )
        .route("/products/physicalstates", get(get_physical_states))
        .route("/products/physicalstates_old", get(get_physical_states_old))
        .route("/products/signalwords", get(get_signal_words))
        .route("/products/signalwords_old", get(get_signal_words_old))
        .route("/products/symbols", get(get_symbols))
        .route("/products/symbols_old", get(get_symbols_old))
        .route(
            "/products/classesofcompounds",
            get(get_classes_of_compounds),
        )
        .route(
            "/products/classesofcompounds_old",
            get(get_classes_of_compounds_old),
        )
        .route("/products/hazardstatements", get(get_hazard_statements))
        .route(
            "/products/hazardstatements_old",
            get(get_hazard_statements_old),
        )
        .route(
            "/products/precautionarystatements",
            get(get_precautionary_statements),
        )
        .route(
            "/products/precautionarystatements_old",
            get(get_precautionary_statements_old),
        )
        .route("/products/categories", get(get_categories))
        .route("/products/categories_old", get(get_categories_old))
        .route("/products/tags", get(get_tags))
        .route("/products/tags_old", get(get_tags_old))
        .route("/products/producers", get(get_producers))
        .route("/products/producers_old", get(get_producers_old))
        .route("/products/producerrefs", get(get_producer_refs))
        .route("/products/producerrefs_old", get(get_producer_refs_old))
        .route("/products/suppliers", get(get_suppliers))
        .route("/products/suppliers_old", get(get_suppliers_old))
        .route("/products/supplierrefs", get(get_supplier_refs))
        .route("/products/supplierrefs_old", get(get_supplier_refs_old))
        .route("/products/producers", post(create_producer))
        .route("/products/suppliers", post(create_supplier))
        //
        .route("/bookmarks/{id}", get(toogle_bookmark))
        //
        .route("/borrows/{id}", get(toogle_borrowing))
        //
        .route("/validate/email/{email}", get(validate_email))
        .route("/validate/casnumber/{cas_number}", get(validate_cas_number))
        .route("/validate/cenumber/{ce_number}", get(validate_ce_number))
        .route(
            "/validate/empiricalformula/{empirical_formula}",
            get(validate_empirical_formula),
        )
        //
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authorize_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_middleware,
        ))
        .layer(Extension(http_client))
        // .layer(middleware::from_fn_with_state(state.clone(), my_middleware))
        .layer(session_layer)
        .layer(cors)
        .with_state(state);

    info!("running server");

    let listener = TcpListener::bind("[::]:8083").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
