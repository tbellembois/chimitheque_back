//! Checks of what a person may write, on data casbin does not see: casbin only
//! checks the item id of the request path, not its body.

use chimitheque_types::{
    permission::{Permission, PermissionItem, PermissionName},
    person::Person,
};
use rusqlite::Connection;

use crate::errors::AppError;

/// What a saved person had before the save; empty for a new person.
#[derive(Debug, Default)]
pub(crate) struct StoredPerson {
    /// The stored permissions.
    pub permissions: Vec<Permission>,
    /// The ids of the entities the person manages.
    pub managed_entity_ids: Vec<u64>,
    /// The ids of the entities the person is a member of.
    pub member_entity_ids: Vec<u64>,
}

/// The person who sends the request.
#[derive(Debug, Default)]
pub(crate) struct Requester {
    /// Whether the requester is an admin.
    pub is_admin: bool,
    /// The ids of the entities the requester manages.
    pub managed_entity_ids: Vec<u64>,
}

/// A permission, without its person.
type Grant = (PermissionName, PermissionItem, Option<u64>);

/// Checks that the requester may save the person.
///
/// - Admins are set at startup: the admin permission can not be given or
///   removed, even by an admin.
/// - Managers are set on the entities, by an admin: the saved person can not
///   change them.
/// - A manager has fixed permissions: they can not be changed.
/// - A non-admin can only change permissions on the entities they manage.
/// - A non-admin can only add people to, or remove them from, the entities they
///   manage.
///
/// Permissions derived by the save are ignored: reading products, reading the
/// entities of the person, and the permissions of their managed entities.
///
/// # Errors
///
/// Returns the reason of a refusal.
pub(crate) fn check_person_save(
    requester: &Requester,
    stored: &StoredPerson,
    person: &Person,
) -> Result<(), String> {
    if let Some(managed_entities) = &person.managed_entities {
        let mut managed_entity_ids: Vec<u64> = managed_entities
            .iter()
            .filter_map(|entity| entity.entity_id)
            .collect();
        managed_entity_ids.sort_unstable();
        managed_entity_ids.dedup();

        if managed_entity_ids != stored.managed_entity_ids {
            return Err("the managers of an entity are set on the entity, by an admin".to_string());
        }
    }

    let stored_grants = explicit_grants(&stored.permissions, &stored.managed_entity_ids);
    let grants = explicit_grants(
        person.permissions.as_deref().unwrap_or_default(),
        &stored.managed_entity_ids,
    );

    // Admins are set at startup, from the ADMINS list: the admin permission can
    // not be given or removed by a save, even by an admin.
    let is_admin_grant = |grant: &Grant| *grant == (PermissionName::All, PermissionItem::All, None);
    if grants.iter().any(is_admin_grant) != stored_grants.iter().any(is_admin_grant) {
        return Err("admins are set at startup, from the ADMINS list".to_string());
    }

    if !stored.managed_entity_ids.is_empty() && !same_grants(&grants, &stored_grants) {
        return Err("the permissions of a manager can not be changed".to_string());
    }

    if !requester.is_admin {
        // Grants the requester can not change: on another entity, or on all
        // entities.
        let is_fixed = |grant: &&Grant| {
            grant
                .2
                .is_none_or(|entity_id| !requester.managed_entity_ids.contains(&entity_id))
        };
        let fixed_grants: Vec<Grant> = grants.iter().filter(is_fixed).cloned().collect();
        let stored_fixed_grants: Vec<Grant> =
            stored_grants.iter().filter(is_fixed).cloned().collect();

        if !same_grants(&fixed_grants, &stored_fixed_grants) {
            return Err(
                "only an admin can change permissions on all entities or on entities you do not manage"
                    .to_string(),
            );
        }

        // Memberships of the entities the requester does not manage.
        let unmanaged = |entity_ids: Vec<u64>| -> Vec<u64> {
            let mut entity_ids: Vec<u64> = entity_ids
                .into_iter()
                .filter(|entity_id| !requester.managed_entity_ids.contains(entity_id))
                .collect();
            entity_ids.sort_unstable();
            entity_ids.dedup();
            entity_ids
        };
        let member_entity_ids: Vec<u64> = person
            .entities
            .iter()
            .flatten()
            .filter_map(|entity| entity.entity_id)
            .collect();

        if unmanaged(member_entity_ids) != unmanaged(stored.member_entity_ids.clone()) {
            return Err(
                "only an admin can add people to, or remove them from, entities you do not manage"
                    .to_string(),
            );
        }
    }

    Ok(())
}

/// Refuses unless the person can write the `item` of the entity.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal, or `AppError::Database`.
pub(crate) fn refuse_unless_can_write_in_entity(
    db_connection: &Connection,
    person_id: u64,
    item: &PermissionItem,
    entity_id: u64,
) -> Result<(), AppError> {
    match chimitheque_db::permission::person_can_write_in_entity(
        db_connection,
        person_id,
        item,
        entity_id,
    ) {
        Ok(true) => Ok(()),
        Ok(false) => Err(AppError::InputValidation(format!(
            "you can not write {item} in entity {entity_id}"
        ))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

/// Refuses unless the person can write the `item` of the entity of the store
/// location.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal or an unknown store
/// location, or `AppError::Database`.
pub(crate) fn refuse_unless_can_write_in_store_location_entity(
    db_connection: &Connection,
    person_id: u64,
    item: &PermissionItem,
    store_location_id: u64,
) -> Result<(), AppError> {
    match chimitheque_db::storelocation::store_location_entity_id(db_connection, store_location_id)
    {
        Ok(Some(entity_id)) => {
            refuse_unless_can_write_in_entity(db_connection, person_id, item, entity_id)
        }
        Ok(None) => Err(AppError::InputValidation(format!(
            "unknown store_location id {store_location_id}"
        ))),
        Err(err) => Err(AppError::Database(err.to_string())),
    }
}

/// Returns the grants of `permissions` not derived by the save.
fn explicit_grants(permissions: &[Permission], managed_entity_ids: &[u64]) -> Vec<Grant> {
    permissions
        .iter()
        .map(|permission| {
            (
                permission.permission_name,
                permission.permission_item.clone(),
                permission.permission_entity,
            )
        })
        .filter(|(name, item, entity)| {
            let reads_products = *name == PermissionName::Read
                && *item == PermissionItem::Products
                && entity.is_none();
            let reads_an_entity = *name == PermissionName::Read
                && *item == PermissionItem::Entities
                && entity.is_some();
            let manages_an_entity = *name == PermissionName::All
                && *item == PermissionItem::All
                && entity.is_some_and(|entity_id| managed_entity_ids.contains(&entity_id));
            !(reads_products || reads_an_entity || manages_an_entity)
        })
        .collect()
}

/// Returns whether both lists have the same grants, in any order.
fn same_grants(a: &[Grant], b: &[Grant]) -> bool {
    a.iter().all(|grant| b.contains(grant)) && b.iter().all(|grant| a.contains(grant))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use chimitheque_types::entity::Entity;

    fn permission(name: PermissionName, item: PermissionItem, entity: Option<u64>) -> Permission {
        Permission {
            permission_name: name,
            permission_item: item,
            permission_entity: entity,
            ..Default::default()
        }
    }

    fn entities(entity_ids: &[u64]) -> Vec<Entity> {
        entity_ids
            .iter()
            .map(|entity_id| Entity {
                entity_id: Some(*entity_id),
                ..Default::default()
            })
            .collect()
    }

    fn person(permissions: Vec<Permission>, managed_entity_ids: Option<&[u64]>) -> Person {
        Person {
            permissions: Some(permissions),
            managed_entities: managed_entity_ids.map(entities),
            ..Default::default()
        }
    }

    fn admin() -> Requester {
        Requester {
            is_admin: true,
            managed_entity_ids: vec![],
        }
    }

    // A manager of entity 1.
    fn manager() -> Requester {
        Requester {
            is_admin: false,
            managed_entity_ids: vec![1],
        }
    }

    const ADMIN_GRANT: (PermissionName, PermissionItem, Option<u64>) =
        (PermissionName::All, PermissionItem::All, None);

    fn grant(grant: (PermissionName, PermissionItem, Option<u64>)) -> Permission {
        permission(grant.0, grant.1, grant.2)
    }

    #[test]
    fn test_only_entities_set_the_managers() {
        let stored = StoredPerson {
            managed_entity_ids: vec![1],
            ..Default::default()
        };

        // Unchanged, or not sent.
        assert!(check_person_save(&manager(), &stored, &person(vec![], Some(&[1]))).is_ok());
        assert!(check_person_save(&manager(), &stored, &person(vec![], None)).is_ok());

        // Made or unmade a manager, even by an admin.
        for requester in [manager(), admin()] {
            assert!(
                check_person_save(&requester, &stored, &person(vec![], Some(&[1, 2]))).is_err()
            );
            assert!(check_person_save(&requester, &stored, &person(vec![], Some(&[]))).is_err());
        }
        assert!(
            check_person_save(
                &admin(),
                &StoredPerson::default(),
                &person(vec![], Some(&[2]))
            )
            .is_err()
        );
    }

    #[test]
    fn test_manager_permissions_are_fixed() {
        // A manager of entity 2, who can also write the storages of entity 3.
        let storages_3 = permission(PermissionName::Write, PermissionItem::Storages, Some(3));
        let stored = StoredPerson {
            permissions: vec![
                permission(PermissionName::All, PermissionItem::All, Some(2)),
                permission(PermissionName::Read, PermissionItem::Products, None),
                permission(PermissionName::Read, PermissionItem::Entities, Some(2)),
                storages_3.clone(),
            ],
            managed_entity_ids: vec![2],
            ..Default::default()
        };

        // Unchanged, the derived permissions sent or not.
        assert!(
            check_person_save(&admin(), &stored, &person(stored.permissions.clone(), None)).is_ok()
        );
        assert!(
            check_person_save(&admin(), &stored, &person(vec![storages_3.clone()], None)).is_ok()
        );

        // Changed, even by an admin.
        assert!(check_person_save(&admin(), &stored, &person(vec![], None)).is_err());
        assert!(
            check_person_save(
                &admin(),
                &stored,
                &person(vec![storages_3, grant(ADMIN_GRANT)], None)
            )
            .is_err()
        );
    }

    #[test]
    fn test_non_admin_grants_on_managed_entities_only() {
        let storages = |entity| permission(PermissionName::Write, PermissionItem::Storages, entity);
        let new_person = StoredPerson::default();

        // On the managed entity 1.
        assert!(
            check_person_save(
                &manager(),
                &new_person,
                &person(vec![storages(Some(1))], None)
            )
            .is_ok()
        );

        // On another entity, on all entities, or admin.
        for permission in [storages(Some(2)), storages(None), grant(ADMIN_GRANT)] {
            assert!(
                check_person_save(&manager(), &new_person, &person(vec![permission], None))
                    .is_err()
            );
        }

        // An admin can grant on any entity.
        assert!(
            check_person_save(
                &admin(),
                &new_person,
                &person(vec![storages(Some(2))], None)
            )
            .is_ok()
        );
    }

    #[test]
    fn test_non_admin_keeps_the_grants_they_do_not_manage() {
        // Write on entity 2 and read on all entities, given by an admin.
        let others = vec![
            permission(PermissionName::Write, PermissionItem::Storages, Some(2)),
            permission(PermissionName::Read, PermissionItem::Storages, None),
        ];
        let stored = StoredPerson {
            permissions: others.clone(),
            ..Default::default()
        };

        // Kept, with a change on the managed entity 1.
        let mut permissions = others.clone();
        permissions.push(permission(
            PermissionName::Write,
            PermissionItem::Products,
            Some(1),
        ));
        assert!(check_person_save(&manager(), &stored, &person(permissions, None)).is_ok());

        // Removed.
        assert!(
            check_person_save(&manager(), &stored, &person(vec![others[0].clone()], None)).is_err()
        );
        assert!(check_person_save(&manager(), &stored, &person(vec![], None)).is_err());
    }

    #[test]
    fn test_non_admin_memberships_of_managed_entities_only() {
        let with_entities = |entity_ids: &[u64]| Person {
            entities: Some(entities(entity_ids)),
            ..Default::default()
        };
        // A member of entities 1 and 2.
        let stored = StoredPerson {
            member_entity_ids: vec![1, 2],
            ..Default::default()
        };

        // Added to or removed from the managed entity 1.
        assert!(check_person_save(&manager(), &stored, &with_entities(&[2])).is_ok());
        assert!(
            check_person_save(&manager(), &StoredPerson::default(), &with_entities(&[1])).is_ok()
        );

        // Added to or removed from entity 3 or 2.
        assert!(check_person_save(&manager(), &stored, &with_entities(&[1, 2, 3])).is_err());
        assert!(check_person_save(&manager(), &stored, &with_entities(&[1])).is_err());
        assert!(check_person_save(&manager(), &stored, &Person::default()).is_err());

        // An admin can.
        assert!(check_person_save(&admin(), &stored, &with_entities(&[3])).is_ok());
    }

    #[test]
    fn test_admin_permission_is_fixed() {
        let admin_error = Err("admins are set at startup, from the ADMINS list".to_string());
        let storages = permission(PermissionName::Write, PermissionItem::Storages, Some(1));

        // Given to a new or an existing person, even by an admin.
        for requester in [admin(), manager()] {
            assert_eq!(
                check_person_save(
                    &requester,
                    &StoredPerson::default(),
                    &person(vec![grant(ADMIN_GRANT)], None)
                ),
                admin_error
            );
        }

        // Removed from an admin, even by an admin.
        let stored_admin = StoredPerson {
            permissions: vec![grant(ADMIN_GRANT)],
            ..Default::default()
        };
        assert_eq!(
            check_person_save(
                &admin(),
                &stored_admin,
                &person(vec![storages.clone()], None)
            ),
            admin_error
        );
        assert_eq!(
            check_person_save(&admin(), &stored_admin, &Person::default()),
            admin_error
        );

        // Kept.
        assert!(
            check_person_save(
                &admin(),
                &stored_admin,
                &person(vec![grant(ADMIN_GRANT), storages], None)
            )
            .is_ok()
        );
    }

    #[test]
    fn test_refuse_unless_can_write() {
        unsafe {
            std::env::set_var("SQLITE_EXTENSION_DIR", "src/extensions");
        };
        let mut db_connection = chimitheque_db::init::connect_test();
        chimitheque_db::init::create_tables(&mut db_connection).unwrap();
        // Person 2 writes the storages of entity 1; store location 10 is in
        // entity 1, store location 20 in entity 2.
        db_connection
            .execute_batch(
                "INSERT INTO person (person_id, person_email) VALUES (2, 'test@example.com');
                 INSERT INTO entity (entity_id, entity_name) VALUES (1, 'entity 1'), (2, 'entity 2');
                 INSERT INTO store_location (store_location_id, store_location_name, entity)
                   VALUES (10, 'room 1', 1), (20, 'room 2', 2);
                 INSERT INTO permission (person, permission_name, permission_item, permission_entity)
                   VALUES (2, 'w', 'storages', 1);",
            )
            .unwrap();
        let storages = PermissionItem::Storages;

        assert!(refuse_unless_can_write_in_entity(&db_connection, 2, &storages, 1).is_ok());
        assert!(matches!(
            refuse_unless_can_write_in_entity(&db_connection, 2, &storages, 2),
            Err(AppError::InputValidation(_))
        ));
        assert!(
            refuse_unless_can_write_in_store_location_entity(&db_connection, 2, &storages, 10)
                .is_ok()
        );
        assert!(matches!(
            refuse_unless_can_write_in_store_location_entity(&db_connection, 2, &storages, 20),
            Err(AppError::InputValidation(_))
        ));
        assert!(matches!(
            refuse_unless_can_write_in_store_location_entity(&db_connection, 2, &storages, 99),
            Err(AppError::InputValidation(_))
        ));
    }
}
