//! Guild cache snapshots and updates.

use kira_protocol::GuildDetails;
use serenity::model::guild::{Guild, PartialGuild};

use crate::state;

pub(crate) fn sort(guilds: &mut [GuildDetails]) {
    guilds.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
}

pub(crate) fn upsert(guilds: &mut Vec<GuildDetails>, guild: GuildDetails) -> bool {
    match guilds.iter_mut().find(|existing| existing.id == guild.id) {
        Some(existing) if *existing == guild => false,
        Some(existing) => {
            *existing = guild;
            sort(guilds);
            true
        }
        None => {
            guilds.push(guild);
            sort(guilds);
            true
        }
    }
}

pub(crate) fn remove(guilds: &mut Vec<GuildDetails>, id: &str) -> bool {
    let previous_len = guilds.len();
    guilds.retain(|guild| guild.id != id);
    guilds.len() != previous_len
}

pub(crate) fn details(guild: &Guild) -> GuildDetails {
    let mut details = GuildDetails::new(guild.id.to_string(), guild.name.clone());
    details.description = guild.description.clone();
    details.icon_url = guild.icon_url();
    details.owner_id = guild.owner_id.to_string();
    details.member_count = guild.member_count;
    details.presence_count = guild.approximate_presence_count;
    details.preferred_locale = guild.preferred_locale.clone();
    details.features = guild.features.clone();
    details
}

pub(crate) fn partial_details(guild: &PartialGuild) -> GuildDetails {
    let mut details = GuildDetails::new(guild.id.to_string(), guild.name.clone());
    details.description = guild.description.clone();
    details.icon_url = guild.icon_url();
    details.owner_id = guild.owner_id.to_string();
    details.member_count = guild.approximate_member_count.unwrap_or_default();
    details.presence_count = guild.approximate_presence_count;
    details.preferred_locale = guild.preferred_locale.clone();
    details.features = guild.features.clone();
    details
}

pub(crate) fn merge_update(
    existing: Option<&GuildDetails>,
    mut details: GuildDetails,
) -> GuildDetails {
    if let Some(existing) = existing {
        details.member_count = existing.member_count;

        if details.presence_count.is_none() {
            details.presence_count = existing.presence_count;
        }
    }

    details
}

pub(crate) fn is_removal(unavailable: bool) -> bool {
    !unavailable
}

pub(crate) fn snapshot(cache: &serenity::cache::Cache, target: Option<u64>) -> Vec<GuildDetails> {
    let mut guilds = cache
        .guilds()
        .into_iter()
        .filter(|id| target.is_none_or(|target| id.get() == target))
        .filter_map(|id| cache.guild(id))
        .map(|guild| details(&guild))
        .collect::<Vec<_>>();

    sort(&mut guilds);
    guilds
}

pub(crate) async fn publish(
    ctx: &serenity::prelude::Context,
    update: impl FnOnce(&mut Vec<GuildDetails>) -> bool,
) {
    let Some(sender) = state::guild_sender(ctx).await else {
        return;
    };

    sender.send_if_modified(update);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_adds_in_name_order() {
        let mut guilds = vec![GuildDetails::new("1", "Beta")];

        assert!(upsert(&mut guilds, GuildDetails::new("2", "Alpha")));
        assert_eq!(
            guilds,
            vec![
                GuildDetails::new("2", "Alpha"),
                GuildDetails::new("1", "Beta"),
            ]
        );
    }

    #[test]
    fn upsert_replaces_the_existing_entry() {
        let mut guilds = vec![GuildDetails::new("1", "Old")];

        assert!(upsert(&mut guilds, GuildDetails::new("1", "New")));
        assert_eq!(guilds, vec![GuildDetails::new("1", "New")]);
    }

    #[test]
    fn upsert_reports_an_unchanged_entry() {
        let mut guilds = vec![GuildDetails::new("1", "Same")];

        assert!(!upsert(&mut guilds, GuildDetails::new("1", "Same")));
        assert_eq!(guilds, vec![GuildDetails::new("1", "Same")]);
    }

    #[test]
    fn remove_deletes_by_id() {
        let mut guilds = vec![
            GuildDetails::new("1", "Alpha"),
            GuildDetails::new("2", "Beta"),
        ];

        assert!(remove(&mut guilds, "1"));
        assert_eq!(guilds, vec![GuildDetails::new("2", "Beta")]);
        assert!(!remove(&mut guilds, "9"));
    }

    #[test]
    fn guild_update_preserves_cached_counts() {
        let mut existing = GuildDetails::new("1", "Alpha");
        existing.member_count = 12;
        existing.presence_count = Some(3);

        let updated = GuildDetails::new("1", "Renamed");

        let merged = merge_update(Some(&existing), updated);

        assert_eq!(merged.name, "Renamed");
        assert_eq!(merged.member_count, 12);
        assert_eq!(merged.presence_count, Some(3));
    }

    #[test]
    fn guild_update_keeps_a_reported_presence_count() {
        let mut existing = GuildDetails::new("1", "Alpha");
        existing.presence_count = Some(2);

        let mut updated = GuildDetails::new("1", "Alpha");
        updated.presence_count = Some(5);

        let merged = merge_update(Some(&existing), updated);
        assert_eq!(merged.presence_count, Some(5));
    }

    #[test]
    fn unavailable_guild_events_are_not_removals() {
        assert!(!is_removal(true));
        assert!(is_removal(false));
    }
}
