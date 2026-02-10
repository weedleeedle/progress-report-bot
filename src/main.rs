use poise::serenity_prelude as serenity;
use presley_bot::commands;
use presley_bot::core;

type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> Result<()> 
{
    env_logger::init();
    log::info!("Starting application");

    let dotenvy_result = dotenvy::dotenv();
    if dotenvy_result.is_err()
    {
        log::warn!("Couldn't load .env file");
    }

    let variables = core::Variables::load_variables()?;
    log::debug!("Loaded variables: {:?}", variables);
    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;
    log::debug!("serenity intents: {:?}", intents);

    let global_command_data = core::GlobalCommandDataBuilder::default()
                                .max_connections(variables.max_connections())
                                .database_url(variables.database_url().to_string())
                                .build().await?;

    log::debug!("Built global_command_data: {:?}", global_command_data);
    // Trigger running migrations on the database
    log::trace!("Running migrations");
    sqlx::migrate!("./migrations")
        .run(global_command_data.get_pool())
        .await?;
    log::trace!("Ran migrations");

    log::debug!("Commands: {:?}", commands::get_commands());

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
                    log::info!("We're in release mode, registering commands globally with Discord");
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }
                else
                {
                    log::info!("We're in debug mode. Not registering commands globally with Discord");
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
