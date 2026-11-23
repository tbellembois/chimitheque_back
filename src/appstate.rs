use casbin::{
    CoreApi, DefaultModel, Enforcer, StringAdapter, function_map::OperatorFunction, rhai::Dynamic,
};
use chimitheque_db::casbin::{
    match_entity_has_members, match_person_is_admin, match_person_is_in_entity,
    match_person_is_manager, match_product_has_storages, match_storage_is_in_entity,
    match_store_location_has_children, match_store_location_has_storages,
    match_store_location_is_in_entity, to_string_adapter,
};
use dashmap::DashMap;
use governor::{
    RateLimiter,
    clock::QuantaClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use r2d2::{self, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;
use tracing::error;

use crate::errors::AppError;

/// Set when a casbin matcher function fails during an enforce call.
///
/// A failed matcher returns `false`, which a negated condition of the policy,
/// e.g. `!matchPersonIsAdmin(...)`, would turn into an allow: the authorize
/// middleware clears it before enforcing and denies the request when it is set.
/// Enforce calls are serialized by the enforcer lock.
pub(crate) static MATCHER_FAILED: AtomicBool = AtomicBool::new(false);

/// Records a failed matcher and returns `false`.
fn matcher_failed() -> bool {
    MATCHER_FAILED.store(true, Ordering::SeqCst);
    false
}

#[derive(Clone)]
pub struct AppState {
    pub db_connection_pool: Arc<Pool<SqliteConnectionManager>>,

    pub casbin_enforcer: Arc<Mutex<Enforcer>>,

    pub rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware>>,

    pub keycloak_base_url: String,
    pub keycloak_client_id: String,
    pub keycloak_realm: String,

    pub pkce_store: Arc<Mutex<DashMap<String, String>>>,
}

pub async fn init_casbin_enforcer(
    arc_enforcer: Arc<Mutex<Enforcer>>,
    arc_db_connection_pool: Arc<Pool<SqliteConnectionManager>>,
) -> Result<(), AppError> {
    let mut casbin_enforcer = arc_enforcer.lock().await;
    let db_connection_pool = match arc_db_connection_pool.get() {
        Ok(db_connection_pool) => db_connection_pool,
        Err(err) => return Err(AppError::DatabasePool(err.to_string())),
    };

    let casbin_string_adapter = to_string_adapter(&db_connection_pool).unwrap();
    let casbin_model = DefaultModel::from_str(include_str!("casbin/policy.conf"))
        .await
        .unwrap();
    let casbin_adapter = StringAdapter::new(casbin_string_adapter.clone());

    *casbin_enforcer = Enforcer::new(casbin_model, casbin_adapter).await.unwrap();

    let db_connection_pool_1 = arc_db_connection_pool.clone();
    let db_connection_pool_2 = db_connection_pool_1.clone();
    let db_connection_pool_3 = db_connection_pool_1.clone();
    let db_connection_pool_4 = db_connection_pool_1.clone();
    let db_connection_pool_5 = db_connection_pool_1.clone();
    let db_connection_pool_6 = db_connection_pool_1.clone();
    let db_connection_pool_7 = db_connection_pool_1.clone();
    let db_connection_pool_8 = db_connection_pool_1.clone();
    let db_connection_pool_9 = db_connection_pool_1.clone();
    let db_connection_pool_10 = db_connection_pool_1.clone();

    casbin_enforcer.add_function(
        "matchProductHasStorages",
        OperatorFunction::Arg1Closure(Arc::new(move |product_id: Dynamic| {
            let product_id: u64 = match product_id.clone().into_string() {
                Ok(product_id) => match product_id.parse::<u64>() {
                    Ok(product_id) => product_id,
                    Err(err) => {
                        error!("failed to parse product ID to u64: {} {}", product_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert product ID into string: {} {}",
                        product_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_1.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_product_has_storages(&db_connection, product_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match product has storages: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchStoreLocationIsInEntity",
        OperatorFunction::Arg2Closure(Arc::new(
            move |store_location_id: Dynamic, entity_id: Dynamic| {
                let store_location_id: u64 = match store_location_id.clone().into_string() {
                    Ok(store_location_id) => match store_location_id.parse::<u64>() {
                        Ok(store_location_id) => store_location_id,
                        Err(err) => {
                            error!(
                                "failed to parse store location ID to u64: {} {}",
                                store_location_id, err
                            );
                            return matcher_failed().into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert store location ID into string: {} {}",
                            store_location_id, err
                        );
                        return matcher_failed().into();
                    }
                };

                let entity_id: u64 = match entity_id.clone().into_string() {
                    Ok(entity_id) => match entity_id.parse::<u64>() {
                        Ok(entity_id) => entity_id,
                        Err(err) => {
                            error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                            return matcher_failed().into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert entity ID into string: {} {}",
                            entity_id, err
                        );
                        return matcher_failed().into();
                    }
                };

                let db_connection = match db_connection_pool_2.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return matcher_failed().into();
                    }
                };

                let result = match match_store_location_is_in_entity(
                    &db_connection,
                    store_location_id,
                    entity_id,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match store location is in entity: {}", err);
                        matcher_failed()
                    }
                };

                result.into()
            },
        )),
    );

    casbin_enforcer.add_function(
        "matchStorageIsInEntity",
        OperatorFunction::Arg2Closure(Arc::new(move |storage_id: Dynamic, entity_id: Dynamic| {
            let storage_id: u64 = match storage_id.clone().into_string() {
                Ok(storage_id) => match storage_id.parse::<u64>() {
                    Ok(storage_id) => storage_id,
                    Err(err) => {
                        error!("failed to parse storage ID to u64: {} {}", storage_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert storage ID into string: {} {}",
                        storage_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let entity_id: u64 = match entity_id.clone().into_string() {
                Ok(entity_id) => match entity_id.parse::<u64>() {
                    Ok(entity_id) => entity_id,
                    Err(err) => {
                        error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert entity ID into string: {} {}",
                        entity_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_3.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_storage_is_in_entity(&db_connection, storage_id, entity_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match storage is in entity: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchStoreLocationHasChildren",
        OperatorFunction::Arg1Closure(Arc::new(move |store_location_id: Dynamic| {
            let store_location_id: u64 = match store_location_id.clone().into_string() {
                Ok(store_location_id) => match store_location_id.parse::<u64>() {
                    Ok(store_location_id) => store_location_id,
                    Err(err) => {
                        error!(
                            "failed to parse store location ID to u64: {} {}",
                            store_location_id, err
                        );
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert store location ID into string: {} {}",
                        store_location_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_4.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_store_location_has_children(&db_connection, store_location_id)
            {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match store location has children: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchStoreLocationHasStorages",
        OperatorFunction::Arg1Closure(Arc::new(move |store_location_id: Dynamic| {
            let store_location_id: u64 = match store_location_id.clone().into_string() {
                Ok(store_location_id) => match store_location_id.parse::<u64>() {
                    Ok(store_location_id) => store_location_id,
                    Err(err) => {
                        error!(
                            "failed to parse store location ID to u64: {} {}",
                            store_location_id, err
                        );
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert store location ID into string: {} {}",
                        store_location_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_5.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_store_location_has_storages(&db_connection, store_location_id)
            {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match store location has storages: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchPersonIsInEntity",
        OperatorFunction::Arg2Closure(Arc::new(move |person_id: Dynamic, entity_id: Dynamic| {
            let person_id: u64 = match person_id.clone().into_string() {
                Ok(person_id) => match person_id.parse::<u64>() {
                    Ok(person_id) => person_id,
                    Err(err) => {
                        error!("failed to parse person ID to u64: {} {}", entity_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert person ID into string: {} {}",
                        person_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let entity_id: u64 = match entity_id.clone().into_string() {
                Ok(entity_id) => match entity_id.parse::<u64>() {
                    Ok(entity_id) => entity_id,
                    Err(err) => {
                        error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert entity ID into string: {} {}",
                        entity_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_6.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_person_is_in_entity(&db_connection, person_id, entity_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match person is in entity: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchPersonIsAdmin",
        OperatorFunction::Arg1Closure(Arc::new(move |person_id: Dynamic| {
            let person_id: u64 = match person_id.clone().into_string() {
                Ok(person_id) => match person_id.parse::<u64>() {
                    Ok(person_id) => person_id,
                    Err(err) => {
                        error!("failed to parse person ID to u64: {} {}", person_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert person ID into string: {} {}",
                        person_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_7.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_person_is_admin(&db_connection, person_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match person is admin: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchPersonIsManager",
        OperatorFunction::Arg1Closure(Arc::new(move |person_id: Dynamic| {
            let person_id: u64 = match person_id.clone().into_string() {
                Ok(person_id) => match person_id.parse::<u64>() {
                    Ok(person_id) => person_id,
                    Err(err) => {
                        error!("failed to parse person ID to u64: {} {}", person_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert person ID into string: {} {}",
                        person_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_8.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_person_is_manager(&db_connection, person_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match person is manager: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchEntityHasMembers",
        OperatorFunction::Arg1Closure(Arc::new(move |entity_id: Dynamic| {
            let entity_id: u64 = match entity_id.clone().into_string() {
                Ok(entity_id) => match entity_id.parse::<u64>() {
                    Ok(entity_id) => entity_id,
                    Err(err) => {
                        error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert entity ID into string: {} {}",
                        entity_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_9.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_entity_has_members(&db_connection, entity_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match entity has members: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    casbin_enforcer.add_function(
        "matchEntityHasStoreLocations",
        OperatorFunction::Arg1Closure(Arc::new(move |entity_id: Dynamic| {
            let entity_id: u64 = match entity_id.clone().into_string() {
                Ok(entity_id) => match entity_id.parse::<u64>() {
                    Ok(entity_id) => entity_id,
                    Err(err) => {
                        error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                        return matcher_failed().into();
                    }
                },
                Err(err) => {
                    error!(
                        "failed to convert entity ID into string: {} {}",
                        entity_id, err
                    );
                    return matcher_failed().into();
                }
            };

            let db_connection = match db_connection_pool_10.get() {
                Ok(db_connection) => db_connection,
                Err(err) => {
                    error!("failed to get database connection pool: {}", err);
                    return matcher_failed().into();
                }
            };

            let result = match match_entity_has_members(&db_connection, entity_id) {
                Ok(result) => result,
                Err(err) => {
                    error!("failed to match entity has store locations: {}", err);
                    matcher_failed()
                }
            };

            result.into()
        })),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    // Person 1 is an admin; people 2 and 5 manage entity 6, where person 7 is
    // a member; person 3 reads the products; person 4 reads the storages of
    // entity 6, where storage 10 is.
    const POLICY: &str = "p, 1, all, all, \n\
                          p, 2, all, all, 6\n\
                          p, 5, all, all, 6\n\
                          p, 3, r, products, \n\
                          p, 4, r, storages, 6\n";

    fn arg1(matches: &'static [&'static str]) -> OperatorFunction {
        OperatorFunction::Arg1Closure(Arc::new(move |id: Dynamic| {
            matches.contains(&id.to_string().as_str()).into()
        }))
    }

    fn arg2(matches: &'static [(&'static str, &'static str)]) -> OperatorFunction {
        OperatorFunction::Arg2Closure(Arc::new(move |id: Dynamic, entity_id: Dynamic| {
            matches
                .contains(&(id.to_string().as_str(), entity_id.to_string().as_str()))
                .into()
        }))
    }

    fn enforcer() -> Enforcer {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let model = DefaultModel::from_str(include_str!("casbin/policy.conf"))
                .await
                .unwrap();
            let mut enforcer = Enforcer::new(model, StringAdapter::new(POLICY))
                .await
                .unwrap();
            enforcer.add_function("matchProductHasStorages", arg1(&[]));
            enforcer.add_function("matchStoreLocationIsInEntity", arg2(&[]));
            enforcer.add_function("matchStorageIsInEntity", arg2(&[("10", "6")]));
            enforcer.add_function("matchStoreLocationHasChildren", arg1(&[]));
            enforcer.add_function("matchStoreLocationHasStorages", arg1(&[]));
            enforcer.add_function("matchPersonIsInEntity", arg2(&[("5", "6"), ("7", "6")]));
            enforcer.add_function("matchPersonIsAdmin", arg1(&["1"]));
            enforcer.add_function("matchPersonIsManager", arg1(&["2", "5"]));
            enforcer.add_function("matchEntityHasMembers", arg1(&[]));
            enforcer.add_function("matchEntityHasStoreLocations", arg1(&[]));
            enforcer
        })
    }

    #[test]
    fn test_policy() {
        let enforcer = enforcer();
        let allowed = |person: &str, action: &str, item: &str, item_id: &str| {
            enforcer.enforce((person, action, item, item_id)).unwrap()
        };

        // A manager updates a member of their entity, not another manager.
        assert!(allowed("2", "u", "people", "7"));
        assert!(!allowed("2", "u", "people", "5"));
        // An admin updates a manager.
        assert!(allowed("1", "u", "people", "5"));

        // Reading the storages of the entity allows toggling a borrowing.
        assert!(allowed("4", "r", "borrows", "10"));
        assert!(allowed("4", "r", "storages", "10"));
        assert!(!allowed("4", "u", "storages", "10"));
        assert!(allowed("4", "r", "stocks", "1"));

        // Reading the products allows bookmarks, not storages.
        assert!(allowed("3", "r", "products", "1"));
        assert!(allowed("3", "r", "bookmarks", "1"));
        assert!(!allowed("3", "c", "products", ""));
        assert!(!allowed("3", "r", "storages", ""));

        // Removed rules: not checked by casbin, refused if asked.
        for item in ["rproducts", "exportproducts", "connecteduser"] {
            assert!(!allowed("3", "r", item, ""), "{item}");
        }
    }
}
