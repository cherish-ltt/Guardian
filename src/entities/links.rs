use crate::entities::{admin_roles, admins, audit_logs, permissions, role_permissions, roles};
use sea_orm::{Linked, RelationDef, RelationTrait};

/// Admin → Roles (via admin_roles)
pub struct AdminToRoles;

impl Linked for AdminToRoles {
    type FromEntity = admins::Entity;
    type ToEntity = roles::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            admin_roles::Relation::Admin.def().rev(),
            admin_roles::Relation::Role.def(),
        ]
    }
}

/// Admin → Permissions (via admin_roles → role_permissions)
pub struct AdminToPermissions;

impl Linked for AdminToPermissions {
    type FromEntity = admins::Entity;
    type ToEntity = permissions::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            admin_roles::Relation::Admin.def().rev(),
            admin_roles::Relation::Role.def(),
            role_permissions::Relation::Role.def().rev(),
            role_permissions::Relation::Permission.def(),
        ]
    }
}

/// Role → Admins (via admin_roles)
pub struct RoleToAdmins;

impl Linked for RoleToAdmins {
    type FromEntity = roles::Entity;
    type ToEntity = admins::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            admin_roles::Relation::Role.def().rev(),
            admin_roles::Relation::Admin.def(),
        ]
    }
}

/// Role → Permissions (via role_permissions)
pub struct RoleToPermissions;

impl Linked for RoleToPermissions {
    type FromEntity = roles::Entity;
    type ToEntity = permissions::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            role_permissions::Relation::Role.def().rev(),
            role_permissions::Relation::Permission.def(),
        ]
    }
}

/// Permission → Roles (via role_permissions)
pub struct PermissionToRoles;

impl Linked for PermissionToRoles {
    type FromEntity = permissions::Entity;
    type ToEntity = roles::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            role_permissions::Relation::Permission.def().rev(),
            role_permissions::Relation::Role.def(),
        ]
    }
}

/// Admin → Audit Logs
pub struct AdminToAuditLogs;

impl Linked for AdminToAuditLogs {
    type FromEntity = admins::Entity;
    type ToEntity = audit_logs::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![audit_logs::Relation::Admin.def().rev()]
    }
}
