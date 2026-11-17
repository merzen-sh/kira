//! Bot presence persistence.

use kira_protocol::{ActivityKind, BotActivity, BotPresence, BotStatus};
use sqlx::FromRow;

use crate::{Database, StoreError};

const PRESENCE_COLUMNS: &str = "status, activity_kind, activity_text";

/// Queries for the bot's stored presence.
#[derive(Clone)]
pub struct PresenceRepository {
    db: Database,
}

impl PresenceRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Read the stored presence configuration.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the query fails or stored values are invalid.
    pub async fn get(&self) -> Result<BotPresence, StoreError> {
        let sql = format!("SELECT {PRESENCE_COLUMNS} FROM bot_presence WHERE id = 1");
        let row = sqlx::query_as::<_, PresenceRow>(&sql)
            .fetch_one(self.db.pool())
            .await?;

        row.try_into()
    }

    /// Save the presence configuration and return the stored value.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the update fails or stored values are invalid.
    pub async fn save(&self, presence: &BotPresence) -> Result<BotPresence, StoreError> {
        let (activity_kind, activity_text) =
            presence.activity.as_ref().map_or((None, None), |activity| {
                (Some(activity.kind.as_str()), Some(activity.text.as_str()))
            });
        let sql = format!(
            "UPDATE bot_presence \
             SET status = ?, activity_kind = ?, activity_text = ?, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = 1 \
             RETURNING {PRESENCE_COLUMNS}"
        );
        let row = sqlx::query_as::<_, PresenceRow>(&sql)
            .bind(presence.status.as_str())
            .bind(activity_kind)
            .bind(activity_text)
            .fetch_one(self.db.pool())
            .await?;

        row.try_into()
    }
}

#[derive(Debug, FromRow)]
struct PresenceRow {
    status: String,
    activity_kind: Option<String>,
    activity_text: Option<String>,
}

impl TryFrom<PresenceRow> for BotPresence {
    type Error = StoreError;

    fn try_from(row: PresenceRow) -> Result<Self, Self::Error> {
        let status = parse_status(&row.status)?;
        let activity = match (row.activity_kind, row.activity_text) {
            (None, None) => None,
            (Some(kind), Some(text)) => Some(BotActivity::new(parse_kind(&kind)?, text)),
            _ => {
                return Err(StoreError::InvalidData(
                    "bot presence activity kind and text must both be set".to_owned(),
                ));
            }
        };

        Ok(BotPresence::new(status, activity))
    }
}

fn parse_status(value: &str) -> Result<BotStatus, StoreError> {
    match value {
        "online" => Ok(BotStatus::Online),
        "idle" => Ok(BotStatus::Idle),
        "dnd" => Ok(BotStatus::DoNotDisturb),
        "invisible" => Ok(BotStatus::Invisible),
        other => Err(StoreError::InvalidData(format!(
            "unknown bot status `{other}`"
        ))),
    }
}

fn parse_kind(value: &str) -> Result<ActivityKind, StoreError> {
    match value {
        "playing" => Ok(ActivityKind::Playing),
        "watching" => Ok(ActivityKind::Watching),
        "listening" => Ok(ActivityKind::Listening),
        "competing" => Ok(ActivityKind::Competing),
        "custom" => Ok(ActivityKind::Custom),
        other => Err(StoreError::InvalidData(format!(
            "unknown bot activity kind `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use kira_protocol::{ActivityKind, BotActivity, BotPresence, BotStatus};

    use crate::{StoreError, memory};

    #[tokio::test]
    async fn default_presence_is_seeded() -> Result<(), StoreError> {
        let presence = memory().await.presence().get().await?;
        assert_eq!(presence, BotPresence::default());
        Ok(())
    }

    #[tokio::test]
    async fn save_persists_status_and_activity() -> Result<(), StoreError> {
        let database = memory().await;
        let expected = BotPresence::new(
            BotStatus::DoNotDisturb,
            Some(BotActivity::new(ActivityKind::Listening, "Kira FM")),
        );

        let saved = database.presence().save(&expected).await?;
        assert_eq!(saved, expected);
        assert_eq!(database.presence().get().await?, expected);
        Ok(())
    }

    #[tokio::test]
    async fn save_clears_activity() -> Result<(), StoreError> {
        let database = memory().await;
        database
            .presence()
            .save(&BotPresence::new(
                BotStatus::Idle,
                Some(BotActivity::new(ActivityKind::Watching, "logs")),
            ))
            .await?;

        let cleared = BotPresence::new(BotStatus::Online, None);
        database.presence().save(&cleared).await?;

        assert_eq!(database.presence().get().await?, cleared);
        Ok(())
    }

    #[tokio::test]
    async fn invalid_stored_status_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let database = memory().await;
        sqlx::query("PRAGMA ignore_check_constraints = ON")
            .execute(database.pool())
            .await?;
        sqlx::query("UPDATE bot_presence SET status = 'away' WHERE id = 1")
            .execute(database.pool())
            .await?;

        assert!(matches!(
            database.presence().get().await,
            Err(StoreError::InvalidData(_))
        ));
        Ok(())
    }
}
