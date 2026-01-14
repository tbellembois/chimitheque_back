use casbin::{CoreApi, Enforcer, function_map::OperatorFunction, rhai::Dynamic};
use chimitheque_db::casbin::{
    match_entity_has_members, match_person_is_admin, match_person_is_in_entity,
    match_person_is_manager, match_product_has_storages, match_storage_is_in_entity,
    match_store_location_has_children, match_store_location_has_storages,
    match_store_location_is_in_entity,
};
use dashmap::DashMap;
use governor::{
    RateLimiter,
    clock::QuantaClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use log::error;
use r2d2::{self, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use std::{ops::Deref, sync::Arc};
use tokio::sync::Mutex;

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

impl AppState {
    pub async fn set_enforcer(&mut self) {
        let db_connection_pool_1 = self.db_connection_pool.clone();
        let db_connection_pool_2 = db_connection_pool_1.clone();
        let db_connection_pool_3 = db_connection_pool_1.clone();
        let db_connection_pool_4 = db_connection_pool_1.clone();
        let db_connection_pool_5 = db_connection_pool_1.clone();
        let db_connection_pool_6 = db_connection_pool_1.clone();
        let db_connection_pool_7 = db_connection_pool_1.clone();
        let db_connection_pool_8 = db_connection_pool_1.clone();
        let db_connection_pool_9 = db_connection_pool_1.clone();
        let db_connection_pool_10 = db_connection_pool_1.clone();

        let mut casbin_enforcer = self.casbin_enforcer.lock().await;

        casbin_enforcer.add_function(
            "matchProductHasStorages",
            OperatorFunction::Arg1Closure(Arc::new(move |product_id: Dynamic| {
                let product_id: u64 = match product_id.clone().into_string() {
                    Ok(product_id) => match product_id.parse::<u64>() {
                        Ok(product_id) => product_id,
                        Err(err) => {
                            error!("failed to parse product ID to u64: {} {}", product_id, err);
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert product ID into string: {} {}",
                            product_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_1.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_product_has_storages(db_connection.deref(), product_id) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match product has storages: {}", err);
                        false
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
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert store location ID into string: {} {}",
                                store_location_id, err
                            );
                            return false.into();
                        }
                    };

                    let entity_id: u64 = match entity_id.clone().into_string() {
                        Ok(entity_id) => match entity_id.parse::<u64>() {
                            Ok(entity_id) => entity_id,
                            Err(err) => {
                                error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert entity ID into string: {} {}",
                                entity_id, err
                            );
                            return false.into();
                        }
                    };

                    let db_connection = match db_connection_pool_2.get() {
                        Ok(db_connection) => db_connection,
                        Err(err) => {
                            error!("failed to get database connection pool: {}", err);
                            return false.into();
                        }
                    };

                    let result = match match_store_location_is_in_entity(
                        db_connection.deref(),
                        store_location_id,
                        entity_id,
                    ) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("failed to match store location is in entity: {}", err);
                            false
                        }
                    };

                    result.into()
                },
            )),
        );

        casbin_enforcer.add_function(
            "matchStorageIsInEntity",
            OperatorFunction::Arg2Closure(Arc::new(
                move |storage_id: Dynamic, entity_id: Dynamic| {
                    let storage_id: u64 = match storage_id.clone().into_string() {
                        Ok(storage_id) => match storage_id.parse::<u64>() {
                            Ok(storage_id) => storage_id,
                            Err(err) => {
                                error!("failed to parse storage ID to u64: {} {}", storage_id, err);
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert storage ID into string: {} {}",
                                storage_id, err
                            );
                            return false.into();
                        }
                    };

                    let entity_id: u64 = match entity_id.clone().into_string() {
                        Ok(entity_id) => match entity_id.parse::<u64>() {
                            Ok(entity_id) => entity_id,
                            Err(err) => {
                                error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert entity ID into string: {} {}",
                                entity_id, err
                            );
                            return false.into();
                        }
                    };

                    let db_connection = match db_connection_pool_3.get() {
                        Ok(db_connection) => db_connection,
                        Err(err) => {
                            error!("failed to get database connection pool: {}", err);
                            return false.into();
                        }
                    };

                    let result = match match_storage_is_in_entity(
                        db_connection.deref(),
                        storage_id,
                        entity_id,
                    ) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("failed to match storage is in entity: {}", err);
                            false
                        }
                    };

                    result.into()
                },
            )),
        );

        casbin_enforcer.add_function(
            "matchStorelocationHasChildren",
            OperatorFunction::Arg1Closure(Arc::new(move |store_location_id: Dynamic| {
                let store_location_id: u64 = match store_location_id.clone().into_string() {
                    Ok(store_location_id) => match store_location_id.parse::<u64>() {
                        Ok(store_location_id) => store_location_id,
                        Err(err) => {
                            error!(
                                "failed to parse store location ID to u64: {} {}",
                                store_location_id, err
                            );
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert store location ID into string: {} {}",
                            store_location_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_4.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_store_location_has_children(
                    db_connection.deref(),
                    store_location_id,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match store location has children: {}", err);
                        false
                    }
                };

                result.into()
            })),
        );

        casbin_enforcer.add_function(
            "matchStorelocationHasStorages",
            OperatorFunction::Arg1Closure(Arc::new(move |store_location_id: Dynamic| {
                let store_location_id: u64 = match store_location_id.clone().into_string() {
                    Ok(store_location_id) => match store_location_id.parse::<u64>() {
                        Ok(store_location_id) => store_location_id,
                        Err(err) => {
                            error!(
                                "failed to parse store location ID to u64: {} {}",
                                store_location_id, err
                            );
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert store location ID into string: {} {}",
                            store_location_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_5.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_store_location_has_storages(
                    db_connection.deref(),
                    store_location_id,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match store location has storages: {}", err);
                        false
                    }
                };

                result.into()
            })),
        );

        casbin_enforcer.add_function(
            "matchPersonIsInEntity",
            OperatorFunction::Arg2Closure(Arc::new(
                move |person_id: Dynamic, entity_id: Dynamic| {
                    let person_id: u64 = match person_id.clone().into_string() {
                        Ok(person_id) => match person_id.parse::<u64>() {
                            Ok(person_id) => person_id,
                            Err(err) => {
                                error!("failed to parse person ID to u64: {} {}", entity_id, err);
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert person ID into string: {} {}",
                                person_id, err
                            );
                            return false.into();
                        }
                    };

                    let entity_id: u64 = match entity_id.clone().into_string() {
                        Ok(entity_id) => match entity_id.parse::<u64>() {
                            Ok(entity_id) => entity_id,
                            Err(err) => {
                                error!("failed to parse entity ID to u64: {} {}", entity_id, err);
                                return false.into();
                            }
                        },
                        Err(err) => {
                            error!(
                                "failed to convert entity ID into string: {} {}",
                                entity_id, err
                            );
                            return false.into();
                        }
                    };

                    let db_connection = match db_connection_pool_6.get() {
                        Ok(db_connection) => db_connection,
                        Err(err) => {
                            error!("failed to get database connection pool: {}", err);
                            return false.into();
                        }
                    };

                    let result = match match_person_is_in_entity(
                        db_connection.deref(),
                        person_id,
                        entity_id,
                    ) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("failed to match person is in entity: {}", err);
                            false
                        }
                    };

                    result.into()
                },
            )),
        );

        casbin_enforcer.add_function(
            "matchPersonIsAdmin",
            OperatorFunction::Arg1Closure(Arc::new(move |person_id: Dynamic| {
                let person_id: u64 = match person_id.clone().into_string() {
                    Ok(person_id) => match person_id.parse::<u64>() {
                        Ok(person_id) => person_id,
                        Err(err) => {
                            error!("failed to parse person ID to u64: {} {}", person_id, err);
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert person ID into string: {} {}",
                            person_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_7.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_person_is_admin(db_connection.deref(), person_id) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match person is admin: {}", err);
                        false
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
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert person ID into string: {} {}",
                            person_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_8.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_person_is_manager(db_connection.deref(), person_id) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match person is manager: {}", err);
                        false
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
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert entity ID into string: {} {}",
                            entity_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_9.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_entity_has_members(db_connection.deref(), entity_id) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match entity has members: {}", err);
                        false
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
                            return false.into();
                        }
                    },
                    Err(err) => {
                        error!(
                            "failed to convert entity ID into string: {} {}",
                            entity_id, err
                        );
                        return false.into();
                    }
                };

                let db_connection = match db_connection_pool_10.get() {
                    Ok(db_connection) => db_connection,
                    Err(err) => {
                        error!("failed to get database connection pool: {}", err);
                        return false.into();
                    }
                };

                let result = match match_entity_has_members(db_connection.deref(), entity_id) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("failed to match entity has store locations: {}", err);
                        false
                    }
                };

                result.into()
            })),
        );
    }
}
