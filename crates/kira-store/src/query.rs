//! Query helpers shared by the repositories.

/// Escape LIKE wildcards so search text matches literally.
pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Convert a page and page size into SQL `LIMIT` and `OFFSET` values.
pub(crate) fn page_offset(page: u32, page_limit: u32) -> (i64, i64) {
    let limit = i64::from(page_limit);
    let offset = i64::from(page.saturating_sub(1)) * limit;
    (limit, offset)
}
