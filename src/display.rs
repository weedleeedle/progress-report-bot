//! This mod manages the appearance of the bot when rendering things like [Report]s

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;

use crate::report::Report;

pub struct ReportListInteractionHandler
{
    ctx_id_string: String,
    prev_button_id: String,
    next_button_id: String,
    report_list: Vec<Report>,
    reports_per_page: usize,
    num_pages: usize,
    current_page: usize,
}

impl ReportListInteractionHandler
{
    pub fn new(ctx: crate::Context<'_>, reports: Vec<Report>, reports_per_page: usize) -> Self
    {
        Self {
            ctx_id_string: ctx.id().to_string(),
            prev_button_id: format!("{}prev", ctx.id().to_string()),
            next_button_id: format!("{}next", ctx.id().to_string()),
            num_pages: reports.chunks(reports_per_page).len(),
            report_list: reports,
            reports_per_page: reports_per_page,
            current_page: 0
        }
    }

    pub async fn listen(mut self, ctx: crate::Context<'_>) -> anyhow::Result<()>
    {
        while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
                .timeout(std::time::Duration::from_secs(3600))
                .await
        {
            if &press.data.custom_id == &self.next_button_id {
                self.current_page += 1;
                self.current_page = self.current_page.min(self.num_pages - 1);
            }
            else if &press.data.custom_id == &self.prev_button_id {
                self.current_page -= 1;
                self.current_page = self.current_page.max(0);
            }
            else {
                continue;
            }

            press.create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(
                            create_reply_for_report_page(CreateEmbed::new(), self.report_list.chunks(self.reports_per_page).nth(self.current_page).unwrap())
                        )
                )
            ).await?;
        }
        Ok(())
    }

    pub fn ctx_id(&self) -> &str
    {
        &self.ctx_id_string
    }

    pub fn prev_button_id(&self) -> &str
    {
        &self.prev_button_id
    }
    
    pub fn next_button_id(&self) -> &str
    {
        &self.next_button_id
    }
}

/// Returns a [CreateReply] which tells poise how to create the embed.
pub fn create_reply_for_reports(builder: CreateReply, ctx: crate::Context<'_>, reports: Vec<Report>, reports_per_page: usize) -> (CreateReply, ReportListInteractionHandler)
{
    // Create unique identifiers
    let ctx_id  = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).label("Prev"),
            serenity::CreateButton::new(&next_button_id).label("Next"),
        ]);
        let mut report_page = reports.chunks(reports_per_page);
        let first_page = report_page.next();
        let builder = match first_page 
        {
            None => {
                builder.content("No progress reports yet! Submit one with `/report`!")
            },
            Some(page) => {
                builder.embed(create_reply_for_report_page(CreateEmbed::new(), &page))
            }
        };
        builder.components(vec![components])
    };

    // Build a component handler here somehow?
    (reply, ReportListInteractionHandler::new(ctx, reports, reports_per_page))
}

fn create_reply_for_report_page(builder: CreateEmbed, reports: &[Report]) -> CreateEmbed
{
    let mut description = String::new();
    for report in reports
    {
        let submission_note = match report.submission_note()
        {
            None => format!(""),
            Some(note) => format!("\n> {}", note)
        };

        description.push_str(
            &format!("{}\n`{} words`{}\n",
                report.timestamp().format("%Y-%m-%d %H:%M"),
                report.total_word_count(),
                submission_note,
            )
        );
    }
    builder.description(description)
}
