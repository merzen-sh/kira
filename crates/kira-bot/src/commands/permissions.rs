//! Reusable server-side permission checks for slash commands.

use serenity::model::permissions::Permissions;
use thiserror::Error;

/// Where a command's permission must be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionScope {
    /// Guild-wide permissions, ignoring channel overwrites.
    Guild,
    /// Permissions in the channel the command was invoked from.
    Channel,
}

/// A permission a command requires from the invoking member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandPermission {
    permission: Permissions,
    label: &'static str,
    scope: PermissionScope,
}

impl CommandPermission {
    /// Require a guild-wide permission.
    #[must_use]
    pub(crate) const fn guild(permission: Permissions, label: &'static str) -> Self {
        Self {
            permission,
            label,
            scope: PermissionScope::Guild,
        }
    }

    /// Require a permission in the current channel.
    #[must_use]
    pub(crate) const fn channel(permission: Permissions, label: &'static str) -> Self {
        Self {
            permission,
            label,
            scope: PermissionScope::Channel,
        }
    }

    /// Scope used to resolve the member's permissions.
    #[must_use]
    pub(crate) const fn scope(self) -> PermissionScope {
        self.scope
    }

    /// Check resolved member permissions against this requirement.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionDenied`] when the member is neither an
    /// administrator nor granted the required permission.
    pub(crate) fn authorize(self, member_permissions: Permissions) -> Result<(), PermissionDenied> {
        if member_permissions.contains(Permissions::ADMINISTRATOR)
            || member_permissions.contains(self.permission)
        {
            Ok(())
        } else {
            Err(PermissionDenied {
                permission: self.label,
            })
        }
    }
}

/// A permission check failed for the invoking member.
#[derive(Debug, Error)]
#[error("you need the **{permission}** permission to use this command")]
pub(crate) struct PermissionDenied {
    permission: &'static str,
}

#[cfg(test)]
mod tests {
    use serenity::model::permissions::Permissions;

    use super::{CommandPermission, PermissionScope};

    const BAN: CommandPermission =
        CommandPermission::guild(Permissions::BAN_MEMBERS, "Ban Members");

    #[test]
    fn grants_the_declared_permission() {
        assert_eq!(BAN.scope(), PermissionScope::Guild);
        assert!(BAN.authorize(Permissions::BAN_MEMBERS).is_ok());
    }

    #[test]
    fn administrator_grants_everything() {
        assert!(BAN.authorize(Permissions::ADMINISTRATOR).is_ok());
    }

    #[test]
    fn missing_permission_names_the_requirement() {
        let error = BAN
            .authorize(Permissions::KICK_MEMBERS)
            .expect_err("ban permission is required");

        assert_eq!(
            error.to_string(),
            "you need the **Ban Members** permission to use this command"
        );
    }
}
