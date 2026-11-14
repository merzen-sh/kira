//! The `/report` slash command and its modal.

use kira_store::NewReport;
use serenity::builder::{
    CreateActionRow, CreateCommand, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal,
};
use serenity::model::application::{
    ActionRowComponent, CommandInteraction, InputTextStyle, ModalInteraction,
};
use serenity::prelude::Context;

use crate::handler::ReportsKey;

/// Name of the `/report` slash command.
pub(super) const NAME: &str = "report";

/// Custom ID of the report modal.
pub(super) const MODAL_ID: &str = "report_modal";

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
        data.get::<ReportsKey>().cloned()
    };

    let Some(store) = store else {
        tracing::error!("Report storage is not available.");
        return "Sorry, the report could not be saved.".to_owned();
    };

    let user_id = modal
        .member
        .as_ref()
        .map_or(modal.user.id, |member| member.user.id)
        .to_string();

    let report = NewReport::new(&user_id, title, steps, details);

    match store.create_report(&report).await {
        Ok(report) => format!("Thanks! Bug report #{} was saved.", report.id),
        Err(error) => {
            tracing::error!("Failed to save bug report: {error:?}");
            "Sorry, the report could not be saved.".to_owned()
        }
    }
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
