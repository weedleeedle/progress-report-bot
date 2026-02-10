use log::debug;
use log::info;
use log::warn;
use poise::serenity_prelude as serenity;
use presley_bot::commands;
use presley_bot::core;

type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> Result<()> 
{
    env_logger::init();
    info!("Starting application");

    let dotenvy_result = dotenvy::dotenv();
    if dotenvy_result.is_err()
    {
        warn!("Couldn't load .env file");
    }

    let variables = core::Variables::load_variables()?;
    debug!("Loaded variables: {:?}", variables);
    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;
    debug!("serenity intents: {:?}", intents);

    let global_command_data = core::GlobalCommandDataBuilder::default()
                                .max_connections(variables.max_connections())
                                .database_url(variables.database_url().to_string())
                                .build().await?;

    debug!("Built global_command_data: {:?}", global_command_data);
    // Trigger running migrations on the database
    trace!("Running migrations");
    sqlx::migrate!("./migrations")
        .run(global_command_data.get_pool())
        .await?;
    trace!("Ran migrations");

    debug!("Commands: {:?}", commands::get_commands());

    let framework = poise::Framework::<core::GlobalCommandData, anyhow::Error>::builder() 
        .options(poise::FrameworkOptions {
            commands: commands::get_commands(),
            prefix_options: poise::PrefixFrameworkOptions {
                mention_as_prefix: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Register commands globally if in release mode
                if cfg!(not(debug_assertions))
                {
                    info!("We're in release mode, registering commands globally with Discord");
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }
                else
                {
                    info!("We're in debug mode. Not registering commands globally with Discord");
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
