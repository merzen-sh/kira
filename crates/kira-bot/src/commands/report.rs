//! The `/report` slash command and its modal.

use kira_protocol::{AuditEntity, SOURCE_BOT};
use kira_store::Database;
use kira_store::audit::NewAuditEvent;
use kira_store::reports::NewReport;
use serde_json::Map;
use serenity::builder::{
    CreateActionRow, CreateCommand, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal,
};
use serenity::model::application::{
    ActionRowComponent, CommandInteraction, InputTextStyle, ModalInteraction,
};
use serenity::prelude::Context;

use crate::state::DatabaseKey;

/// Name of the `/report` slash command.
pub(super) const NAME: &str = "report";

/// Custom ID of the report modal.
pub(super) const MODAL_ID: &str = "report_modal";

/// Action recorded when a user submits a bug report.
const ACTION_REPORT_CREATED: &str = "report.created";

const TITLE_ID: &str = "report_title";
const STEPS_ID: &str = "report_steps";
const DETAILS_ID: &str = "report_details";

pub(super) fn command() -> CreateCommand {
    CreateCommand::new(NAME).description("Report a bug to the Kira dashboard.")
}

/// Open the report modal for a `/report` command.
pub(super) async fn open(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), Box<serenity::Error>> {
    let modal = CreateModal::new(MODAL_ID, "Report a bug").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, "What is broken?", TITLE_ID)
                .placeholder("Short description")
                .max_length(100),
        ),
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "What did you do?", STEPS_ID)
                .placeholder("Steps to reproduce")
                .max_length(4000),
        ),
        CreateActionRow::InputText(
            CreateInputText::new(
                InputTextStyle::Paragraph,
                "What happened, and what did you expect?",
                DETAILS_ID,
            )
            .max_length(4000),
        ),
    ]);

    command
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

/// Save a submitted report modal and acknowledge it.
pub(super) async fn submit(
    ctx: &Context,
    modal: &ModalInteraction,
) -> Result<(), Box<serenity::Error>> {
    let content = match submitted_fields(modal) {
        Some((title, steps, details)) => save(ctx, modal, title, steps, details).await,
        None => "Please fill in every field.".to_owned(),
    };

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );

    modal.create_response(&ctx.http, response).await?;

    Ok(())
}

async fn save(
    ctx: &Context,
    modal: &ModalInteraction,
    title: &str,
    steps: &str,
    details: &str,
) -> String {
    let store = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().cloned()
    };

    let Some(store) = store else {
        tracing::error!("Report storage is not available.");
        return "Sorry, the report could not be saved.".to_owned();
    };

    match save_from_modal(&store, modal, title, steps, details).await {
        Ok(id) => format!("Thanks! Bug report #{id} was saved."),
        Err(error) => {
            tracing::error!("Failed to save bug report: {error:?}");
            "Sorry, the report could not be saved.".to_owned()
        }
    }
}

async fn save_from_modal(
    store: &Database,
    modal: &ModalInteraction,
    title: &str,
    steps: &str,
    details: &str,
) -> Result<i64, kira_store::StoreError> {
    let user = modal
        .member
        .as_ref()
        .map_or(&modal.user, |member| &member.user);
    let user_id = user.id.to_string();

    save_report(store, &user_id, &user.name, title, steps, details).await
}

async fn save_report(
    store: &Database,
    user_id: &str,
    user_name: &str,
    title: &str,
    steps: &str,
    details: &str,
) -> Result<i64, kira_store::StoreError> {
    let report = store
        .reports()
        .create_report(&NewReport::new(user_id, title, steps, details))
        .await?;

    let actor = AuditEntity::new("user", Some(user_id.to_owned()), Some(user_name.to_owned()));
    let target = AuditEntity::new(
        "report",
        Some(report.id.to_string()),
        Some(report.title.clone()),
    );
    let metadata = Map::new();
    let event = NewAuditEvent::new(
        ACTION_REPORT_CREATED,
        SOURCE_BOT,
        &actor,
        Some(&target),
        &metadata,
    );

    if let Err(error) = store.audit().record(&event).await {
        tracing::error!(
            action = event.action,
            source = event.source,
            actor_kind = event.actor.kind,
            actor_id = user_id,
            target_kind = event.target.map_or("-", |target| target.kind.as_str()),
            "failed to record report audit event: {error:?}"
        );
    }

    Ok(report.id)
}

fn submitted_fields(modal: &ModalInteraction) -> Option<(&str, &str, &str)> {
    let title = field(modal, TITLE_ID)?;
    let steps = field(modal, STEPS_ID)?;
    let details = field(modal, DETAILS_ID)?;
    Some((title, steps, details))
}

fn field<'a>(modal: &'a ModalInteraction, custom_id: &str) -> Option<&'a str> {
    modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|component| match component {
            ActionRowComponent::InputText(input) if input.custom_id == custom_id => input
                .value
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::{save_from_modal, save_report};
    use kira_store::Database;
    use serenity::model::application::ModalInteraction;

    fn modal() -> Result<ModalInteraction, serde_json::Error> {
        serde_json::from_value(serde_json::json!({
            "id": "1",
            "application_id": "2",
            "data": {
                "custom_id": "report_modal",
                "components": [
                    { "type": 1, "components": [
                        { "type": 4, "custom_id": "report_title", "style": 1, "label": "Title", "value": "Crash" }
                    ] },
                    { "type": 1, "components": [
                        { "type": 4, "custom_id": "report_steps", "style": 2, "label": "Steps", "value": "open it" }
                    ] },
                    { "type": 1, "components": [
                        { "type": 4, "custom_id": "report_details", "style": 2, "label": "Details", "value": "boom" }
                    ] }
                ]
            },
            "guild_id": null,
            "channel": null,
            "channel_id": "3",
            "member": null,
            "user": {
                "id": "42",
                "username": "alice",
                "global_name": null,
                "avatar": null,
                "banner": null,
                "accent_color": null,
                "locale": null,
                "verified": null,
                "email": null,
                "public_flags": null,
                "member": null,
                "primary_guild": null,
                "avatar_decoration_data": null,
                "collectibles": null
            },
            "token": "token",
            "version": 1,
            "message": null,
            "app_permissions": null,
            "locale": "en-US",
            "guild_locale": null,
            "entitlements": [],
            "attachment_size_limit": 4000
        }))
    }

    #[tokio::test]
    async fn submitted_report_records_an_audit_event() -> Result<(), Box<dyn std::error::Error>> {
        let store = Database::connect("sqlite::memory:").await?;
        let modal = modal()?;

        let id = save_from_modal(&store, &modal, "Crash", "open it", "boom").await?;
        let page = store
            .audit()
            .list_events(1, 20, None, None, None, None)
            .await?;

        assert_eq!(page.total, 1);
        let event = &page.events[0];
        let target_id = id.to_string();
        assert_eq!(event.action, "report.created");
        assert_eq!(event.source, "bot");
        assert_eq!(event.actor.kind, "user");
        assert_eq!(event.actor.id.as_deref(), Some("42"));
        assert_eq!(event.actor.name.as_deref(), Some("alice"));
        assert_eq!(
            event
                .target
                .as_ref()
                .and_then(|target| target.id.as_deref()),
            Some(target_id.as_str())
        );
        assert_eq!(
            event
                .target
                .as_ref()
                .and_then(|target| target.name.as_deref()),
            Some("Crash")
        );
        assert!(event.metadata.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn direct_report_save_still_records_an_event() -> Result<(), Box<dyn std::error::Error>> {
        let store = Database::connect("sqlite::memory:").await?;

        save_report(&store, "42", "alice", "Crash", "open it", "boom").await?;
        let page = store
            .audit()
            .list_events(1, 20, None, None, None, None)
            .await?;

        assert_eq!(page.total, 1);
        Ok(())
    }
}
