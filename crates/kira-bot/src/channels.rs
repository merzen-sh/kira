//! Channel cache snapshots and updates.

use kira_protocol::ChannelDetails;
use serenity::model::channel::GuildChannel;
use serenity::prelude::Context;

use crate::state;

pub(crate) fn details(channel: &GuildChannel) -> ChannelDetails {
    let mut details = ChannelDetails::new(
        channel.id.to_string(),
        channel.guild_id.to_string(),
        channel.name.clone(),
        channel.kind.name(),
    );
    details.position = channel.position;
    details.parent_id = channel.parent_id.map(|id| id.to_string());
    details
}

pub(crate) fn sort(channels: &mut [ChannelDetails]) {
    channels.sort_by(|a, b| {
        a.guild_id
            .cmp(&b.guild_id)
            .then(a.position.cmp(&b.position))
            .then_with(|| a.name.cmp(&b.name))
    });
}

pub(crate) fn snapshot(cache: &serenity::cache::Cache, target: Option<u64>) -> Vec<ChannelDetails> {
    let mut channels = Vec::new();

    for id in cache.guilds() {
        if target.is_some_and(|target| id.get() != target) {
            continue;
        }

        if let Some(guild) = cache.guild(id) {
            channels.extend(guild.channels.values().map(details));
        }
    }

    sort(&mut channels);
    channels
}

pub(crate) async fn publish(ctx: &Context) {
    let sender = {
        let data = ctx.data.read().await;
        data.get::<state::ChannelsKey>().cloned()
    };

    if let Some(sender) = sender {
        let target = state::target_guild(ctx).await;
        sender.send_replace(snapshot(&ctx.cache, target));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_sorted_by_guild_position_and_name() {
        let mut late = ChannelDetails::new("3", "2", "general", "text");
        late.position = 1;
        let mut early = ChannelDetails::new("2", "1", "voice", "voice");
        early.position = 5;
        let first = ChannelDetails::new("1", "1", "general", "text");

        let mut channels = vec![late, early, first.clone()];
        sort(&mut channels);

        assert_eq!(first, channels[0]);
        assert_eq!("2", channels[1].id);
        assert_eq!("3", channels[2].id);
    }
}
