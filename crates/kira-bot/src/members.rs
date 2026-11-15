//! Guild member display helpers.

/// Prefer the guild nickname, then the global display name, then the username.
pub(crate) fn display_name(
    nick: Option<&str>,
    global_name: Option<&str>,
    user_name: &str,
) -> String {
    nick.or(global_name).unwrap_or(user_name).to_owned()
}
