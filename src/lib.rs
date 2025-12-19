pub mod appstate;
pub mod errors;
pub mod handlers;
pub mod utils;

use crate::{
    appstate::AppState,
    errors::AppError,
    handlers::person::{create_update_person, delete_person, get_people},
    handlers::store_location::{
        create_update_store_location, delete_store_location, get_store_locations,
    },
    utils::get_chimitheque_person_id_from_headers,
};
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    extract::{Request, State},
    http::{HeaderMap, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use axum_oidc::{
    EmptyAdditionalClaims, OidcAuthLayer, OidcClaims, OidcLoginLayer, OidcRpInitiatedLogout,
    error::MiddlewareError,
};
use casbin::{CoreApi, DefaultModel, Enforcer, StringAdapter};
use chimitheque_db::{
    casbin::to_string_adapter,
    init::{init_db, update_ghs_statements},
};
use chimitheque_types::requestfilter::RequestFilter;
use chrono::Local;
use http::Method;
use log::debug;
use r2d2::{self};
use r2d2_sqlite::SqliteConnectionManager;
use std::{
    env,
    ops::{Deref, DerefMut},
    path::Path,
    sync::{Arc, Mutex},
};
use std::{io::Write, os::unix::fs::MetadataExt};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};
use url::Url;

// Extract OIDC claims from the request.
// Insert the authenticated user id and email into the request headers.
async fn authenticate_middleware(
    State(state): State<AppState>,
    mayerr_claims: Result<OidcClaims<EmptyAdditionalClaims>, axum_oidc::error::ExtractorError>,
    mut request: Request,
    next: Next,
) -> Response {
    debug!("authenticate_middleware");

    let db_connection = state.db_connection_pool.get().unwrap();

    // Get the claims from the request.
    let Ok(claims) = mayerr_claims else {
        return AppError::ClaimsRetrieval(mayerr_claims.err().unwrap().to_string()).into_response();
    };

    // Get the email from the claims.
    let Some(claims_email) = claims.email() else {
        return AppError::MissingEmailInClaims.into_response();
    };

    // Convert the email to a string.
    let person_email = claims_email.to_string();

    // Get the person from the database.
    let (people, _) = match chimitheque_db::person::get_people(
        db_connection.deref(),
        RequestFilter {
            person_email: Some(person_email),
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
    let request_full = format!("http://localhost{}", request_uri.path());

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

    // Check that the first path segment is expected.
    if ![String::from("store_locations")].contains(&item) {
        return AppError::InvalidFirstPathSegment(Some(item)).into_response();
    };

    debug!("chimitheque_person_id: {}", chimitheque_person_id);
    debug!("request_action: {}", request_action);
    debug!("item: {}", item);
    debug!("item_id: {}", item_id);

    // Get the casbin enforcer from the state object.
    // TODO: https://github.com/tokio-rs/axum/discussions/2458
    {
        let casbin_enforcer = state.casbin_enforcer;

        // Then check the permissions.
        let casbin_enforcer = match casbin_enforcer.lock() {
            Ok(enforcer) => enforcer,
            Err(err) => return AppError::CasbinEnforcerLockFailed(err.to_string()).into_response(),
        };
        match casbin_enforcer.enforce((chimitheque_person_id, request_action, item, item_id)) {
            Ok(true) => (),
            Ok(false) => return AppError::PermissionDenied.into_response(),
            Err(err) => return AppError::CasbinError(err.to_string()).into_response(),
        };
    }

    next.run(request).await
}

pub async fn run(
    app_url: String,
    db_path: String,
    issuer: String,
    client_id: String,
    client_secret: Option<String>,
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

    // Load SQLite extensions directory.
    let sql_extension_dir = env::var("SQLITE_EXTENSION_DIR")
        .expect("Missing SQLITE_EXTENSION_DIR environment variable.");
    let sql_extension_regex = Path::new(sql_extension_dir.as_str()).join("regexp.so");

    // Create DB pool.
    let manager = SqliteConnectionManager::file(db_path.clone());
    let db_connection_pool = r2d2::Pool::builder().build(manager).unwrap();

    // Load extensions.
    let mut db_connection = db_connection_pool.get().unwrap();

    unsafe {
        db_connection
            .load_extension(sql_extension_regex, None)
            .unwrap();
    }

    // Check that DB file exist, create if not.
    if Path::new(&db_path).metadata().unwrap().size() == 0 {
        init_db(db_connection.deref_mut()).unwrap();
    } else {
        // Updating statements on already existing DB - panic on failure.
        let db_transaction = db_connection.transaction().unwrap();

        update_ghs_statements(&db_transaction).unwrap();

        db_transaction.commit().unwrap();
    }

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(120)));

    let oidc_login_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            e.into_response()
        }))
        .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

    let oidc_auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            e.into_response()
        }))
        .layer(
            OidcAuthLayer::<EmptyAdditionalClaims>::discover_client(
                Uri::from_maybe_shared(app_url).expect("valid APP_URL"),
                issuer,
                client_id,
                client_secret,
                vec![],
            )
            .await
            .unwrap(),
        );

    // Initialize the Casbin toolkit.
    let casbin_string_adapter = to_string_adapter(db_connection.deref()).unwrap();
    let casbin_model = DefaultModel::from_file("casbin/policy.conf").await.unwrap();

    // TODO: FIXME
    let casbin_adapter = StringAdapter::new(casbin_string_adapter.clone());
    let casbin_enforcer = Arc::new(Mutex::new(
        Enforcer::new(casbin_model, casbin_adapter).await.unwrap(),
    ));

    let mut state = AppState {
        casbin_enforcer,
        db_connection_pool: Arc::new(db_connection_pool),
    };

    state.set_enforcer();

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
    //
    //
    //
    // router.Handle("/{item:store_locations}/{id}", secureChain.Then(env.AppMiddleware(env.UpdateStoreLocationHandler))).Methods("PUT")
    // router.Handle("/{item:store_locations}", secureChain.Then(env.AppMiddleware(env.CreateStoreLocationHandler))).Methods("POST")
    // router.Handle("/{item:store_locations}/{id}", secureChain.Then(env.AppMiddleware(env.DeleteStoreLocationHandler))).Methods("DELETE")

    let app = Router::new()
        .route("/foo", get(authenticated))
        .route("/logout", get(logout))
        //
        .route("/store_locations", get(get_store_locations))
        .route("/store_locations/{id}", get(get_store_locations))
        .route("/store_locations/{id}", put(create_update_store_location))
        .route("/store_locations", post(create_update_store_location))
        .route("/store_locations/{id}", delete(delete_store_location))
        //
        .route("/people", get(get_people))
        .route("/people/{id}", get(get_people))
        .route("/people/{id}", put(create_update_person))
        .route("/people", post(create_update_person))
        .route("/people/{id}", delete(delete_person))
        //
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authorize_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate_middleware,
        ))
        // .route_layer(middleware::from_fn(authenticate_middleware))
        .layer(oidc_login_service)
        .route("/bar", get(maybe_authenticated))
        .layer(oidc_auth_service)
        .layer(session_layer)
        .with_state(state);

    let listener = TcpListener::bind("[::]:8083").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn authenticated(claims: OidcClaims<EmptyAdditionalClaims>) -> impl IntoResponse {
    format!("Hello {}", claims.subject().as_str())
}

#[axum::debug_handler]
async fn maybe_authenticated(
    claims: Result<OidcClaims<EmptyAdditionalClaims>, axum_oidc::error::ExtractorError>,
) -> impl IntoResponse {
    if let Ok(claims) = claims {
        format!(
            "Hello {}! You are already logged in from another Handler.",
            claims.subject().as_str()
        )
    } else {
        "Hello anon!".to_string()
    }
}

async fn logout(logout: OidcRpInitiatedLogout) -> impl IntoResponse {
    logout.with_post_logout_redirect(Uri::from_static("https://pfzetto.de"))
}
