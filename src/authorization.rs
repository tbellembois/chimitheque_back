//! The permission checks.
//!
//! - [`authorize`] checks the request path, before the handler, in the
//!   authorize middleware: the item, the action and the item id.
//! - The other functions check the request body, in the handlers.

use chimitheque_types::{
    permission::{Permission, PermissionItem, PermissionName},
    person::Person,
    product::Product,
};
use http::Method;
use rusqlite::Connection;

use crate::errors::AppError;

/// The action of a request, from its HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// `GET`.
    Read,
    /// `POST`.
    Create,
    /// `PUT`.
    Update,
    /// `DELETE`.
    Delete,
}

impl Action {
    /// Returns the action of the method, or `None` for another method.
    pub(crate) fn from_method(method: &Method) -> Option<Self> {
        match *method {
            Method::GET => Some(Self::Read),
            Method::POST => Some(Self::Create),
            Method::PUT => Some(Self::Update),
            Method::DELETE => Some(Self::Delete),
            _ => None,
        }
    }

    /// Whether the action writes.
    fn writes(self) -> bool {
        self != Self::Read
    }
}

/// The item id of the request path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathId {
    /// No id, e.g. `/storages`.
    NoId,
    /// A numeric id, e.g. `/storages/12`.
    Number(u64),
    /// Another id, e.g. `/products/pubchemautocomplete/ethanol`.
    Text,
}

impl PathId {
    /// Returns the id of the path parameter, if any.
    pub(crate) fn parse(id: Option<&str>) -> Self {
        match id {
            None | Some("") => Self::NoId,
            Some(id) => id.parse().map_or(Self::Text, Self::Number),
        }
    }
}

/// The database, and the person who sends the request.
pub(crate) struct Ctx<'a> {
    /// The database connection.
    pub db: &'a Connection,
    /// The id of the person who sends the request.
    pub person_id: u64,
}

/// Returns whether the person may perform the `action` on the `item` (the
/// first segment of the request path, e.g. `storages`), with the path `id`.
///
/// One line per rule: anything not listed is refused. A rule matching any id
/// (`_`) does not use it. The rights come from the roles (admin, manager,
/// member) and the permissions, see `chimitheque_db::permission`.
///
/// # Errors
///
/// Returns `AppError::Database` if a query fails.
// Kept as a table, one line per rule, even when two rules are the same.
#[rustfmt::skip]
#[allow(clippy::match_same_arms)]
pub(crate) fn authorize(ctx: &Ctx, item: &str, action: Action, id: PathId) -> Result<bool, AppError> {
    use Action::{Create, Delete, Read, Update};
    use PathId::{NoId, Number};

    match (item, action, id) {
        // item                    action           id
        ("products",              Create,          _)          => writes_products(ctx),
        ("products",              Read,            _)          => reads_products(ctx),
        ("products",              Update,          Number(id)) => update_product(ctx, id),
        ("products",              Delete,          Number(id)) => delete_product(ctx, id),

        ("storages",              Create,          _)          => storages(ctx, Create, None),
        ("storages",              Read,            NoId)       => storages(ctx, Read, None),
        ("storages" | "borrows",  Read,            Number(id)) => storage(ctx, Read, id),
        ("storages",              Update | Delete, Number(id)) => storage(ctx, action, id),

        ("store_locations",       Create,          _)          => manages(ctx, None),
        ("store_locations",       Read,            NoId)       => storages(ctx, Read, None),
        ("store_locations",       Read,            Number(id)) => read_store_location(ctx, id),
        ("store_locations",       Update,          Number(id)) => update_store_location(ctx, id),
        ("store_locations",       Delete,          Number(id)) => delete_store_location(ctx, id),

        ("people",                Create,          _)          => manages(ctx, None),
        ("people",                Read,            NoId)       => belongs(ctx, None),
        ("people",                Read,            Number(id)) => read_person(ctx, id),
        ("people",                Update,          Number(id)) => update_person(ctx, id),
        ("people",                Delete,          Number(id)) => delete_person(ctx, id),

        ("entities",              Create,          _)          => is_admin(ctx, ctx.person_id),
        ("entities",              Read,            NoId)       => belongs(ctx, None),
        ("entities",              Read,            Number(id)) => belongs(ctx, Some(id)),
        ("entities",              Update,          _)          => is_admin(ctx, ctx.person_id),
        ("entities",              Delete,          Number(id)) => delete_entity(ctx, id),

        ("bookmarks",             _,               _)          => reads_products(ctx),

        ("stocks",                _,               _)          => storages(ctx, Read, None),

        _ => Ok(false),
    }
}

/// A restricted product is written by the people who can read it.
fn update_product(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    Ok(writes_products(ctx)?
        && (!db(chimitheque_db::product::product_is_restricted(ctx.db, id))?
            || reads_restricted_products(ctx)?))
}

/// A product with storages, archived and history ones included, can not be
/// deleted.
fn delete_product(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    Ok(update_product(ctx, id)? && !db(chimitheque_db::product::product_has_storages(ctx.db, id))?)
}

/// The storage, in the entity of its store location. An unknown storage is in
/// no entity: the admin only.
fn storage(ctx: &Ctx, action: Action, id: u64) -> Result<bool, AppError> {
    match db(chimitheque_db::storage::storage_entity_id(ctx.db, id))? {
        Some(entity_id) => storages(ctx, action, Some(entity_id)),
        None => is_admin(ctx, ctx.person_id),
    }
}

/// The entity of the store location; `None` for an unknown store location.
fn store_location_entity(ctx: &Ctx, id: u64) -> Result<Option<u64>, AppError> {
    db(chimitheque_db::storelocation::store_location_entity_id(
        ctx.db, id,
    ))
}

/// Read with the storages of its entity. An unknown store location: the admin
/// only.
fn read_store_location(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    match store_location_entity(ctx, id)? {
        Some(entity_id) => storages(ctx, Action::Read, Some(entity_id)),
        None => is_admin(ctx, ctx.person_id),
    }
}

/// Written by the managers of its entity. An unknown store location: the admin
/// only.
fn update_store_location(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    match store_location_entity(ctx, id)? {
        Some(entity_id) => manages(ctx, Some(entity_id)),
        None => is_admin(ctx, ctx.person_id),
    }
}

/// A store location with children or storages can not be deleted.
fn delete_store_location(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    Ok(update_store_location(ctx, id)?
        && !db(chimitheque_db::storelocation::store_location_has_children(
            ctx.db, id,
        ))?
        && !db(chimitheque_db::storelocation::store_location_has_storages(
            ctx.db, id,
        ))?)
}

/// Reading a person: by the people of their entities; a person with no entity
/// by the admins and the managers.
fn read_person(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    let Some(entity_ids) = person_entity_ids(ctx, id)? else {
        return Ok(false);
    };
    if entity_ids.is_empty() {
        return manages(ctx, None);
    }
    for entity_id in entity_ids {
        if belongs(ctx, Some(entity_id))? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Updating a person: not yourself, nor an admin; by an admin, or by a manager
/// of one of their entities (any manager for a person with no entity), except
/// another manager.
fn update_person(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    let Some(entity_ids) = person_entity_ids(ctx, id)? else {
        return Ok(false);
    };
    if id == ctx.person_id || is_admin(ctx, id)? {
        return Ok(false);
    }
    if is_admin(ctx, ctx.person_id)? {
        return Ok(true);
    }
    if is_manager(ctx, id)? {
        return Ok(false);
    }
    if entity_ids.is_empty() {
        return manages(ctx, None);
    }
    for entity_id in entity_ids {
        if manages(ctx, Some(entity_id))? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Deleting a person: by an admin, not yourself, a manager or an admin.
fn delete_person(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    Ok(id != ctx.person_id
        && is_admin(ctx, ctx.person_id)?
        && !is_manager(ctx, id)?
        && !is_admin(ctx, id)?)
}

/// Deleting an entity: by an admin, without members or store locations.
fn delete_entity(ctx: &Ctx, id: u64) -> Result<bool, AppError> {
    Ok(is_admin(ctx, ctx.person_id)?
        && !db(chimitheque_db::entity::entity_has_members(ctx.db, id))?
        && !db(chimitheque_db::entity::entity_has_store_locations(
            ctx.db, id,
        ))?)
}

/// The entities the person is a member of, or manages, or `None` for an
/// unknown person.
fn person_entity_ids(ctx: &Ctx, id: u64) -> Result<Option<Vec<u64>>, AppError> {
    if !db(chimitheque_db::person::person_exists(ctx.db, id))? {
        return Ok(None);
    }
    let mut entity_ids = db(chimitheque_db::person::member_entity_ids(ctx.db, id))?;
    entity_ids.extend(db(chimitheque_db::person::managed_entity_ids(ctx.db, id))?);
    entity_ids.sort_unstable();
    entity_ids.dedup();
    Ok(Some(entity_ids))
}

/// Whether the requester can read, or write for another action, the storages
/// of the entity, or of any entity for `None`.
fn storages(ctx: &Ctx, action: Action, entity_id: Option<u64>) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_can_access_storages(
        ctx.db,
        ctx.person_id,
        action.writes(),
        entity_id,
    ))
}

/// Whether the requester is an admin, or manages the entity, or any entity for
/// `None`.
fn manages(ctx: &Ctx, entity_id: Option<u64>) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_manages_entity(
        ctx.db,
        ctx.person_id,
        entity_id,
    ))
}

/// Whether the requester is an admin, or belongs to the entity, or to any
/// entity for `None`.
fn belongs(ctx: &Ctx, entity_id: Option<u64>) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_belongs_to_entity(
        ctx.db,
        ctx.person_id,
        entity_id,
    ))
}

/// Whether the requester can read the products.
fn reads_products(ctx: &Ctx) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_can_read_products(
        ctx.db,
        ctx.person_id,
    ))
}

/// Whether the requester can write the products.
fn writes_products(ctx: &Ctx) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_can_write_products(
        ctx.db,
        ctx.person_id,
    ))
}

/// Whether the requester can read the restricted products.
fn reads_restricted_products(ctx: &Ctx) -> Result<bool, AppError> {
    db(chimitheque_db::permission::person_can_read_restricted_products(ctx.db, ctx.person_id))
}

/// Whether the person is an admin.
fn is_admin(ctx: &Ctx, person_id: u64) -> Result<bool, AppError> {
    db(chimitheque_db::person::is_person_admin(ctx.db, person_id))
}

/// Whether the person manages an entity.
fn is_manager(ctx: &Ctx, person_id: u64) -> Result<bool, AppError> {
    Ok(!db(chimitheque_db::person::managed_entity_ids(
        ctx.db, person_id,
    ))?
    .is_empty())
}

/// Turns a database error into `AppError::Database`.
fn db<T>(result: Result<T, Box<dyn std::error::Error + Send + Sync>>) -> Result<T, AppError> {
    result.map_err(|err| AppError::Database(err.to_string()))
}

/// What a saved person had before the save; empty for a new person.
#[derive(Debug, Default)]
pub(crate) struct StoredPerson {
    /// The stored permissions.
    pub permissions: Vec<Permission>,
    /// The ids of the entities the person manages.
    pub managed_entity_ids: Vec<u64>,
    /// The ids of the entities the person is a member of.
    pub member_entity_ids: Vec<u64>,
    /// Whether the person is an admin.
    pub is_admin: bool,
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

/// Drops the permissions the saved person can not have any more: on the
/// storages of an entity they are not a member of, and all of them without an
/// entity. A person removed from an entity loses its permissions.
pub(crate) fn drop_permissions_outside_entities(person: &mut Person) {
    let member_entity_ids = member_entity_ids(person);
    if let Some(permissions) = &mut person.permissions {
        if member_entity_ids.is_empty() {
            permissions.clear();
        }
        permissions.retain(|permission| {
            permission
                .permission_entity
                .is_none_or(|entity_id| member_entity_ids.contains(&entity_id))
        });
    }
}

/// Checks that the requester may save the person, after
/// [`drop_permissions_outside_entities`].
///
/// - Managers are set on the entities, by an admin: the saved person can not
///   change them.
/// - Admins and managers have no permission: their rights come from their
///   role.
/// - A non-admin can only change the storages permissions of the entities they
///   manage, and the products permissions of the people of those entities.
/// - A non-admin can only add people to, or remove them from, the entities they
///   manage.
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

    let stored_grants = grants(&stored.permissions);
    let grants = grants(person.permissions.as_deref().unwrap_or_default());

    if (stored.is_admin || !stored.managed_entity_ids.is_empty()) && !grants.is_empty() {
        return Err(
            "an admin or a manager has no permission: their rights come from their role"
                .to_string(),
        );
    }

    if !requester.is_admin {
        let manages = |entity_id: &u64| requester.managed_entity_ids.contains(entity_id);
        let member_entity_ids = member_entity_ids(person);

        // The products permissions, of a person of a managed entity, before or
        // after the save.
        let in_a_managed_entity = member_entity_ids
            .iter()
            .chain(&stored.member_entity_ids)
            .any(manages);

        // The grants the requester can not change.
        let is_fixed = |grant: &&Grant| match grant.2 {
            Some(entity_id) => !manages(&entity_id),
            None => !in_a_managed_entity,
        };
        let fixed_grants: Vec<Grant> = grants.iter().filter(is_fixed).cloned().collect();
        let stored_fixed_grants: Vec<Grant> =
            stored_grants.iter().filter(is_fixed).cloned().collect();

        if !same_grants(&fixed_grants, &stored_fixed_grants) {
            return Err(
                "only an admin can change the permissions on the entities you do not manage, or of their people"
                    .to_string(),
            );
        }

        // Memberships of the entities the requester does not manage.
        let unmanaged = |entity_ids: &[u64]| -> Vec<u64> {
            let mut entity_ids: Vec<u64> = entity_ids
                .iter()
                .filter(|entity_id| !manages(entity_id))
                .copied()
                .collect();
            entity_ids.sort_unstable();
            entity_ids.dedup();
            entity_ids
        };

        if unmanaged(&member_entity_ids) != unmanaged(&stored.member_entity_ids) {
            return Err(
                "only an admin can add people to, or remove them from, entities you do not manage"
                    .to_string(),
            );
        }
    }

    Ok(())
}

/// Refuses unless the person can write the storages of the entity of the store
/// location.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal or an unknown store
/// location, or `AppError::Database`.
pub(crate) fn refuse_unless_can_write_storages_in(
    db_connection: &Connection,
    person_id: u64,
    store_location_id: u64,
) -> Result<(), AppError> {
    let entity_id = known_store_location_entity(db_connection, store_location_id)?;
    if db(chimitheque_db::permission::person_can_access_storages(
        db_connection,
        person_id,
        true,
        Some(entity_id),
    ))? {
        Ok(())
    } else {
        Err(AppError::InputValidation(format!(
            "you can not write the storages of entity {entity_id}"
        )))
    }
}

/// Refuses unless the person is an admin or manages the entity.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal, or `AppError::Database`.
pub(crate) fn refuse_unless_manages_entity(
    db_connection: &Connection,
    person_id: u64,
    entity_id: u64,
) -> Result<(), AppError> {
    if db(chimitheque_db::permission::person_manages_entity(
        db_connection,
        person_id,
        Some(entity_id),
    ))? {
        Ok(())
    } else {
        Err(AppError::InputValidation(format!(
            "you do not manage entity {entity_id}"
        )))
    }
}

/// Refuses unless the person is an admin or manages the entity of the store
/// location.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal or an unknown store
/// location, or `AppError::Database`.
pub(crate) fn refuse_unless_manages_store_location_entity(
    db_connection: &Connection,
    person_id: u64,
    store_location_id: u64,
) -> Result<(), AppError> {
    let entity_id = known_store_location_entity(db_connection, store_location_id)?;
    refuse_unless_manages_entity(db_connection, person_id, entity_id)
}

/// Refuses to write a restricted product, or to restrict a product, unless the
/// person can read the restricted products.
///
/// # Errors
///
/// Returns `AppError::InputValidation` on a refusal, or `AppError::Database`.
pub(crate) fn refuse_unless_can_write_product(
    db_connection: &Connection,
    person_id: u64,
    product: &Product,
) -> Result<(), AppError> {
    let restricted = product.product_restricted
        || match product.product_id {
            Some(product_id) => db(chimitheque_db::product::product_is_restricted(
                db_connection,
                product_id,
            ))?,
            None => false,
        };
    if restricted
        && !db(
            chimitheque_db::permission::person_can_read_restricted_products(
                db_connection,
                person_id,
            ),
        )?
    {
        return Err(AppError::InputValidation(
            "you can not write the restricted products".to_string(),
        ));
    }
    Ok(())
}

/// The entity of the store location.
fn known_store_location_entity(
    db_connection: &Connection,
    store_location_id: u64,
) -> Result<u64, AppError> {
    db(chimitheque_db::storelocation::store_location_entity_id(
        db_connection,
        store_location_id,
    ))?
    .ok_or_else(|| {
        AppError::InputValidation(format!("unknown store_location id {store_location_id}"))
    })
}

/// The ids of the entities the saved person is a member of.
fn member_entity_ids(person: &Person) -> Vec<u64> {
    person
        .entities
        .iter()
        .flatten()
        .filter_map(|entity| entity.entity_id)
        .collect()
}

/// Returns the grants of the permissions.
fn grants(permissions: &[Permission]) -> Vec<Grant> {
    permissions
        .iter()
        .map(|permission| {
            (
                permission.permission_name,
                permission.permission_item.clone(),
                permission.permission_entity,
            )
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
    use PermissionItem::{Products, RestrictedProducts, Storages};
    use PermissionName::{Read, Write};
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

    /// A saved person, member of the entities, with the permissions.
    fn person(member_entity_ids: &[u64], permissions: Vec<Permission>) -> Person {
        Person {
            entities: Some(entities(member_entity_ids)),
            permissions: Some(permissions),
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

    /// A stored person, member of the entities, with the permissions.
    fn stored(member_entity_ids: &[u64], permissions: Vec<Permission>) -> StoredPerson {
        StoredPerson {
            permissions,
            member_entity_ids: member_entity_ids.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn test_only_entities_set_the_managers() {
        let stored = StoredPerson {
            managed_entity_ids: vec![1],
            member_entity_ids: vec![1],
            ..Default::default()
        };
        let with_managed = |managed: Option<&[u64]>| Person {
            managed_entities: managed.map(entities),
            ..person(&[1], vec![])
        };

        // Unchanged, or not sent.
        assert!(check_person_save(&manager(), &stored, &with_managed(Some(&[1]))).is_ok());
        assert!(check_person_save(&manager(), &stored, &with_managed(None)).is_ok());

        // Made or unmade a manager, even by an admin.
        for requester in [manager(), admin()] {
            assert!(check_person_save(&requester, &stored, &with_managed(Some(&[1, 2]))).is_err());
            assert!(check_person_save(&requester, &stored, &with_managed(Some(&[]))).is_err());
        }
        assert!(
            check_person_save(
                &admin(),
                &StoredPerson::default(),
                &with_managed(Some(&[2]))
            )
            .is_err()
        );
    }

    #[test]
    fn test_admins_and_managers_have_no_permission() {
        let stored_manager = StoredPerson {
            managed_entity_ids: vec![2],
            member_entity_ids: vec![2],
            ..Default::default()
        };
        let stored_admin = StoredPerson {
            is_admin: true,
            ..stored(&[2], vec![])
        };

        for stored in [stored_manager, stored_admin] {
            assert!(check_person_save(&admin(), &stored, &person(&[2], vec![])).is_ok());
            assert!(
                check_person_save(
                    &admin(),
                    &stored,
                    &person(&[2], vec![permission(Write, Storages, Some(2))])
                )
                .is_err()
            );
            assert!(
                check_person_save(
                    &admin(),
                    &stored,
                    &person(&[2], vec![permission(Read, RestrictedProducts, None)])
                )
                .is_err()
            );
        }
    }

    #[test]
    fn test_non_admin_storages_on_managed_entities_only() {
        let new_person = StoredPerson::default();
        let storages = |entity_id| permission(Write, Storages, Some(entity_id));

        // On the managed entity 1.
        assert!(
            check_person_save(&manager(), &new_person, &person(&[1], vec![storages(1)])).is_ok()
        );

        // On another entity, even for a person of the managed entity.
        assert!(
            check_person_save(&manager(), &new_person, &person(&[1, 2], vec![storages(2)]))
                .is_err()
        );

        // An admin can, on any entity.
        assert!(
            check_person_save(&admin(), &new_person, &person(&[1, 2], vec![storages(2)])).is_ok()
        );
    }

    #[test]
    fn test_non_admin_products_of_the_people_of_managed_entities_only() {
        let products = || {
            vec![
                permission(Write, Products, None),
                permission(Read, RestrictedProducts, None),
            ]
        };

        // A person of the managed entity 1, before or after the save.
        assert!(
            check_person_save(
                &manager(),
                &StoredPerson::default(),
                &person(&[1], products())
            )
            .is_ok()
        );
        assert!(
            check_person_save(
                &manager(),
                &stored(&[1, 2], products()),
                &person(&[2], vec![])
            )
            .is_ok()
        );
        assert!(
            check_person_save(
                &manager(),
                &stored(&[2], vec![]),
                &person(&[1, 2], products())
            )
            .is_ok()
        );

        // A person of another entity only.
        assert!(
            check_person_save(&manager(), &stored(&[2], vec![]), &person(&[2], products()))
                .is_err()
        );
        assert!(
            check_person_save(&manager(), &stored(&[2], products()), &person(&[2], vec![]))
                .is_err()
        );
        // Unchanged.
        assert!(
            check_person_save(
                &manager(),
                &stored(&[2], products()),
                &person(&[2], products())
            )
            .is_ok()
        );
    }

    #[test]
    fn test_non_admin_keeps_the_storages_they_do_not_manage() {
        // A member of entities 1 and 2, writing the storages of entity 2, given
        // by an admin.
        let others = vec![permission(Write, Storages, Some(2))];
        let stored = stored(&[1, 2], others.clone());

        // Kept, with a change on the managed entity 1.
        let mut permissions = others.clone();
        permissions.push(permission(Read, Storages, Some(1)));
        assert!(check_person_save(&manager(), &stored, &person(&[1, 2], permissions)).is_ok());

        // Changed or removed.
        assert!(
            check_person_save(
                &manager(),
                &stored,
                &person(&[1, 2], vec![permission(Read, Storages, Some(2))])
            )
            .is_err()
        );
        assert!(check_person_save(&manager(), &stored, &person(&[1, 2], vec![])).is_err());
    }

    #[test]
    fn test_non_admin_memberships_of_managed_entities_only() {
        // A member of entities 1 and 2.
        let stored = stored(&[1, 2], vec![]);

        // Added to or removed from the managed entity 1.
        assert!(check_person_save(&manager(), &stored, &person(&[2], vec![])).is_ok());
        assert!(
            check_person_save(&manager(), &StoredPerson::default(), &person(&[1], vec![])).is_ok()
        );

        // Added to or removed from entity 3 or 2.
        assert!(check_person_save(&manager(), &stored, &person(&[1, 2, 3], vec![])).is_err());
        assert!(check_person_save(&manager(), &stored, &person(&[1], vec![])).is_err());
        assert!(check_person_save(&manager(), &stored, &Person::default()).is_err());

        // An admin can.
        assert!(check_person_save(&admin(), &stored, &person(&[3], vec![])).is_ok());
    }

    #[test]
    fn test_drop_permissions_outside_entities() {
        let all = vec![
            permission(Write, Storages, Some(1)),
            permission(Read, Storages, Some(2)),
            permission(Write, Products, None),
            permission(Read, RestrictedProducts, None),
        ];
        let kept = |member_entity_ids: &[u64]| -> Vec<Grant> {
            let mut person = person(member_entity_ids, all.clone());
            drop_permissions_outside_entities(&mut person);
            grants(&person.permissions.unwrap())
        };

        // The storages of the entities the person is a member of.
        assert_eq!(kept(&[1, 2]), grants(&all));
        assert_eq!(
            kept(&[1]),
            [
                (Write, Storages, Some(1)),
                (Write, Products, None),
                (Read, RestrictedProducts, None)
            ]
        );
        assert_eq!(
            kept(&[3]),
            [(Write, Products, None), (Read, RestrictedProducts, None)]
        );
        // None without an entity.
        assert_eq!(kept(&[]), []);
        let mut no_entity = Person {
            permissions: Some(all.clone()),
            ..Default::default()
        };
        drop_permissions_outside_entities(&mut no_entity);
        assert_eq!(no_entity.permissions.unwrap().len(), 0);
    }

    #[test]
    fn test_action_and_path_id() {
        assert_eq!(Action::from_method(&Method::GET), Some(Action::Read));
        assert_eq!(Action::from_method(&Method::POST), Some(Action::Create));
        assert_eq!(Action::from_method(&Method::PUT), Some(Action::Update));
        assert_eq!(Action::from_method(&Method::DELETE), Some(Action::Delete));
        assert_eq!(Action::from_method(&Method::PATCH), None);

        assert_eq!(PathId::parse(None), PathId::NoId);
        assert_eq!(PathId::parse(Some("")), PathId::NoId);
        assert_eq!(PathId::parse(Some("12")), PathId::Number(12));
        assert_eq!(PathId::parse(Some("ethanol")), PathId::Text);
        assert_eq!(PathId::parse(Some("-1")), PathId::Text);
    }

    /// People:
    /// - 1 admin
    /// - 2 manager of Chemistry (1), 3 member of Chemistry: writes its storages
    /// - 4 manager of Biology (2), 5 member of Biology: reads its storages and
    ///   the restricted products
    /// - 6 no entity
    /// - 7 member of Chemistry and Biology: reads their storages
    /// - 8 member of Chemistry: writes the products
    /// - 9 member of Physics (3), with no permission
    ///
    /// Entities: 1 Chemistry, 2 Biology, 3 Physics, 4 Geology (a store
    /// location, no member), 5 Astronomy (empty).
    ///
    /// Store locations: 1 Room (Chemistry) > 2 Shelf (Chemistry), 3 Freezer
    /// (Biology), 4 Cave (Geology), 5 Empty (Chemistry).
    ///
    /// Products: 1 with storage 10 on the Shelf, 2 without storage, 3
    /// restricted, without storage.
    fn authorize_fixture() -> Connection {
        unsafe {
            std::env::set_var("SQLITE_EXTENSION_DIR", "src/extensions");
        };
        let mut db_connection = chimitheque_db::init::connect_test();
        chimitheque_db::init::create_tables(&mut db_connection).unwrap();
        db_connection
            .execute_batch(
                "INSERT OR IGNORE INTO person (person_id, person_email) VALUES
                   (1, 'admin@example.com'), (2, 'chemistry.manager@example.com'),
                   (3, 'chemistry.user@example.com'), (4, 'biology.manager@example.com'),
                   (5, 'biology.user@example.com'), (6, 'new.user@example.com'),
                   (7, 'all.reader@example.com'), (8, 'product.editor@example.com'),
                   (9, 'physics.user@example.com');
                 INSERT OR IGNORE INTO admin (admin_person_id) VALUES (1);
                 INSERT INTO entity (entity_id, entity_name) VALUES
                   (1, 'Chemistry'), (2, 'Biology'), (3, 'Physics'), (4, 'Geology'), (5, 'Astronomy');
                 INSERT INTO personentities (personentities_person_id, personentities_entity_id)
                   VALUES (2, 1), (3, 1), (4, 2), (5, 2), (7, 1), (7, 2), (8, 1), (9, 3);
                 INSERT INTO entitypeople (entitypeople_entity_id, entitypeople_person_id)
                   VALUES (1, 2), (2, 4);
                 INSERT INTO permission (person, permission_name, permission_item, permission_entity) VALUES
                   (3, 'w', 'storages', 1),
                   (5, 'r', 'storages', 2), (5, 'r', 'rproducts', NULL),
                   (7, 'r', 'storages', 1), (7, 'r', 'storages', 2),
                   (8, 'w', 'products', NULL);
                 INSERT INTO store_location (store_location_id, store_location_name, entity, store_location)
                   VALUES (1, 'Room', 1, NULL), (2, 'Shelf', 1, 1), (3, 'Freezer', 2, NULL),
                          (4, 'Cave', 4, NULL), (5, 'Empty', 1, NULL);
                 INSERT INTO name (name_id, name_label) VALUES (1, 'WATER'), (2, 'ETHANOL'), (3, 'ACETONE');
                 INSERT INTO product (product_id, product_type, product_restricted, name, person)
                   VALUES (1, 'chem', 0, 1, 1), (2, 'chem', 0, 2, 1), (3, 'chem', 1, 3, 1);
                 INSERT INTO storage (storage_id, storage_creation_date, storage_modification_date,
                                      storage_entry_date, storage_quantity, person, product, store_location)
                   VALUES (10, 1705276800, 1705276800, 1705276800, 1, 3, 1, 2);",
            )
            .unwrap();
        db_connection
    }

    #[test]
    fn test_body_checks() {
        let db = authorize_fixture();

        // Writing the storages of the entity of the store location.
        assert!(refuse_unless_can_write_storages_in(&db, 3, 2).is_ok());
        assert!(refuse_unless_can_write_storages_in(&db, 4, 3).is_ok());
        assert!(refuse_unless_can_write_storages_in(&db, 1, 3).is_ok());
        for (person_id, store_location_id) in [(3, 3), (5, 3), (7, 2), (2, 3), (3, 99)] {
            assert!(
                matches!(
                    refuse_unless_can_write_storages_in(&db, person_id, store_location_id),
                    Err(AppError::InputValidation(_))
                ),
                "{person_id} {store_location_id}"
            );
        }

        // Managing the entity, or the entity of the store location.
        assert!(refuse_unless_manages_entity(&db, 2, 1).is_ok());
        assert!(refuse_unless_manages_entity(&db, 1, 2).is_ok());
        assert!(refuse_unless_manages_entity(&db, 2, 2).is_err());
        assert!(refuse_unless_manages_entity(&db, 3, 1).is_err());
        assert!(refuse_unless_manages_store_location_entity(&db, 2, 1).is_ok());
        assert!(refuse_unless_manages_store_location_entity(&db, 2, 3).is_err());
        assert!(refuse_unless_manages_store_location_entity(&db, 2, 99).is_err());

        // Restricted products: by the people who can read them.
        let product = |product_id: Option<u64>, restricted| Product {
            product_id,
            product_restricted: restricted,
            ..Default::default()
        };
        assert!(refuse_unless_can_write_product(&db, 8, &product(None, false)).is_ok());
        assert!(refuse_unless_can_write_product(&db, 8, &product(Some(2), false)).is_ok());
        assert!(refuse_unless_can_write_product(&db, 8, &product(None, true)).is_err());
        assert!(refuse_unless_can_write_product(&db, 8, &product(Some(3), false)).is_err());
        for person_id in [1, 2, 5] {
            assert!(
                refuse_unless_can_write_product(&db, person_id, &product(Some(3), true)).is_ok(),
                "{person_id}"
            );
        }
    }

    #[test]
    fn test_authorize() {
        use Action::{Create, Delete, Update};
        use PathId::{NoId, Number, Text};

        let db_connection = authorize_fixture();
        let allowed = |person_id: u64, item: &str, action: Action, id: PathId| {
            let ctx = Ctx {
                db: &db_connection,
                person_id,
            };
            authorize(&ctx, item, action, id).unwrap()
        };
        let read = Action::Read;

        // (person, item, action, id, allowed)
        let cases = [
            // Products: read by the people with an entity, written by the
            // managers and product.editor.
            (6, "products", read, NoId, false),
            (9, "products", read, NoId, true),
            (3, "products", read, Number(1), true),
            (3, "products", read, Text, true),
            (3, "products", Create, NoId, false),
            (3, "products", Update, Number(2), false),
            (8, "products", Create, NoId, true),
            (8, "products", Create, Number(1), true),
            (8, "products", Update, Number(2), true),
            (8, "products", Delete, Number(2), true),
            (2, "products", Update, Number(2), true),
            (1, "products", Update, Number(2), true),
            (8, "products", Update, NoId, false),
            // A product with storages can not be deleted.
            (8, "products", Delete, Number(1), false),
            (1, "products", Delete, Number(1), false),
            // A restricted product, by the people who can read it.
            (8, "products", Update, Number(3), false),
            (8, "products", Delete, Number(3), false),
            (2, "products", Update, Number(3), true),
            (1, "products", Delete, Number(3), true),
            // Storages: on the entity of their store location, given by a
            // role or a permission.
            (3, "storages", read, NoId, true),
            (3, "storages", Create, NoId, true),
            (3, "storages", read, Number(10), true),
            (3, "storages", Update, Number(10), true),
            (3, "storages", Delete, Number(10), true),
            (5, "storages", read, NoId, true),
            (5, "storages", read, Number(10), false),
            (5, "storages", Update, Number(10), false),
            (7, "storages", read, Number(10), true),
            (7, "storages", Update, Number(10), false),
            (6, "storages", read, NoId, false),
            (9, "storages", read, NoId, false),
            (8, "storages", Create, NoId, false),
            (2, "storages", Delete, Number(10), true),
            // A manager has no access to the storages of another entity.
            (4, "storages", read, Number(10), false),
            (4, "storages", Delete, Number(10), false),
            // An unknown storage: the admin only.
            (3, "storages", read, Number(999), false),
            (7, "storages", read, Number(999), false),
            (1, "storages", read, Number(999), true),
            // An id is required, and must be a number.
            (1, "storages", Update, NoId, false),
            (1, "storages", read, Text, false),
            // Reading the storages allows toggling a borrowing.
            (3, "borrows", read, Number(10), true),
            (7, "borrows", read, Number(10), true),
            (5, "borrows", read, Number(10), false),
            (3, "borrows", read, NoId, false),
            // Store locations: read with the storages of their entity, written
            // by its managers.
            (3, "store_locations", read, NoId, true),
            (3, "store_locations", read, Number(1), true),
            (3, "store_locations", read, Number(3), false),
            (3, "store_locations", Create, NoId, false),
            (3, "store_locations", Update, Number(1), false),
            (9, "store_locations", read, NoId, false),
            (6, "store_locations", read, NoId, false),
            (2, "store_locations", Create, NoId, true),
            (2, "store_locations", read, Number(1), true),
            (2, "store_locations", Update, Number(1), true),
            (2, "store_locations", read, Number(3), false),
            (2, "store_locations", Update, Number(3), false),
            (2, "store_locations", Delete, Number(5), true),
            (2, "store_locations", Delete, Number(3), false),
            (2, "store_locations", Update, Number(999), false),
            (1, "store_locations", Update, Number(999), true),
            // A store location with children or storages can not be deleted.
            (2, "store_locations", Delete, Number(1), false),
            (2, "store_locations", Delete, Number(2), false),
            (1, "store_locations", Delete, Number(2), false),
            // People: read by the people of their entities; a person with no
            // entity by the admins and the managers.
            (2, "people", read, NoId, true),
            (2, "people", Create, NoId, true),
            (2, "people", read, Number(3), true),
            (2, "people", read, Number(5), false),
            (2, "people", read, Number(6), true),
            (2, "people", read, Number(7), true),
            (3, "people", read, NoId, true),
            (3, "people", read, Number(2), true),
            (3, "people", read, Number(6), false),
            (3, "people", Create, NoId, false),
            (3, "people", Update, Number(8), false),
            (9, "people", read, NoId, true),
            (9, "people", read, Number(3), false),
            (6, "people", read, NoId, false),
            // An unknown person.
            (2, "people", read, Number(999), false),
            (1, "people", read, Number(999), false),
            (1, "people", Update, Number(999), false),
            // Updated by an admin, or a manager of one of their entities; not
            // yourself, nor an admin; a manager by an admin only.
            (2, "people", Update, Number(3), true),
            (2, "people", Update, Number(6), true),
            (2, "people", Update, Number(7), true),
            (4, "people", Update, Number(7), true),
            (2, "people", Update, Number(5), false),
            (2, "people", Update, Number(2), false),
            (2, "people", Update, Number(4), false),
            (2, "people", Update, Number(1), false),
            (1, "people", Update, Number(4), true),
            (1, "people", Update, Number(1), false),
            // Deleted by an admin, not yourself, a manager or an admin.
            (1, "people", Delete, Number(3), true),
            (1, "people", Delete, Number(4), false),
            (1, "people", Delete, Number(1), false),
            (2, "people", Delete, Number(3), false),
            // Entities: read by their people, written by an admin.
            (1, "entities", Create, NoId, true),
            (1, "entities", Update, Number(1), true),
            (2, "entities", Create, NoId, false),
            (2, "entities", Update, Number(1), false),
            (2, "entities", read, NoId, true),
            (2, "entities", read, Number(1), true),
            (2, "entities", read, Number(2), false),
            (7, "entities", read, Number(2), true),
            (9, "entities", read, Number(3), true),
            (9, "entities", read, Number(1), false),
            (1, "entities", read, Number(2), true),
            (6, "entities", read, NoId, false),
            (1, "entities", Delete, Number(5), true),
            (2, "entities", Delete, Number(5), false),
            // An entity with members or store locations can not be deleted.
            (1, "entities", Delete, Number(1), false),
            (1, "entities", Delete, Number(3), false),
            (1, "entities", Delete, Number(4), false),
            // Reading the products allows bookmarks.
            (9, "bookmarks", read, Number(1), true),
            (6, "bookmarks", read, Number(1), false),
            // Reading storages allows the stocks.
            (5, "stocks", read, Number(1), true),
            (7, "stocks", read, Number(1), true),
            (2, "stocks", read, Number(1), true),
            (9, "stocks", read, Number(1), false),
            (6, "stocks", read, Number(1), false),
            // Not a protected item: refused if asked.
            (1, "exportproducts", read, NoId, false),
            (1, "rproducts", read, NoId, false),
            (1, "connecteduser", read, NoId, false),
        ];

        for (person_id, item, action, id, expected) in cases {
            assert_eq!(
                allowed(person_id, item, action, id),
                expected,
                "{person_id} {item} {action:?} {id:?}"
            );
        }

        // The admin can do everything else.
        for (item, action, id) in [
            ("products", Create, NoId),
            ("storages", Delete, Number(10)),
            ("store_locations", Delete, Number(5)),
            ("people", read, Number(6)),
        ] {
            assert!(allowed(1, item, action, id), "{item} {action:?} {id:?}");
        }
    }
}
