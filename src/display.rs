//! This mod manages the appearance of the bot when rendering things like [Report]s

use poise::{serenity_prelude::CreateEmbed};
use crate::Context;

use anyhow::Result;
use crate::report::Report;

/// Returns a [CreateEmbed] which tells poise how to create the embed.
pub fn create_embed_for_reports(builder: CreateEmbed, reports: &[Report], reports_per_page: usize) -> CreateEmbed
{
    let mut report_page = reports.chunks(reports_per_page);
    let first_page = report_page.next();
    let builder = match first_page 
    {
        None => {
            builder.description("No progress reports yet! Submit one with `/report`!")
        },
        Some(page) => {
            create_embed_for_report_page(builder, &page)
        }
    };

    builder
}

/// Sets a [CreateEmbed] description to be a page of reports.
fn create_embed_for_report_page(builder: CreateEmbed, reports: &[Report]) -> CreateEmbed
{
    let mut description = String::new();
    for report in reports
    {
        description.push_str(&format!("{}\n`{} words`\n", report.timestamp(), report.total_word_count()));
    }
    builder.description(description)
}
