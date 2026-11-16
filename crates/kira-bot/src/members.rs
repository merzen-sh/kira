//! Guild member display helpers.

/// Prefer the guild nickname, then the global display name, then the username.
pub(crate) fn display_name<'a>(
    nick: Option<&'a str>,
    global_name: Option<&'a str>,
    user_name: &'a str,
) -> &'a str {
    nick.or(global_name).unwrap_or(user_name)
}
