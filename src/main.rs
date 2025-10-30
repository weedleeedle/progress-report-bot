use log::warn;
use poise::serenity_prelude as serenity;
use progress_report_bot::commands;
use progress_report_bot::core;

type Result<T> = anyhow::Result<T>;
type Context<'a> = poise::Context<'a, (), anyhow::Error>;

#[tokio::main]
async fn main() -> Result<()> 
{
    env_logger::init();

    let dotenvy_result = dotenvy::dotenv();
    if dotenvy_result.is_err()
    {
        warn!("Couldn't load .env file");
    }

    let variables = core::Variables::load_variables()?;
    let intents = serenity::GatewayIntents::non_privileged() & serenity::GatewayIntents::MESSAGE_CONTENT;

    let global_command_data = core::GlobalCommandDataBuilder::new()
                                .max_connections(variables.max_connections())
                                .database_url(variables.database_url().to_string())
                                .build().await?;

    let framework = poise::Framework::<core::GlobalCommandData, anyhow::Error>::builder() 
        .options(poise::FrameworkOptions {
            commands: commands::get_commands(),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Register commands globally if in release mode
                if cfg!(not(debug_assertions))
                {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }
                //global_command_data.set_client(&framework.client());
                Ok(global_command_data)
            })
        })
        .build();
    
    let client = serenity::ClientBuilder::new(variables.token(), intents)
        .framework(framework)
        .await;

    client?.start().await?;
    Ok(())
}
