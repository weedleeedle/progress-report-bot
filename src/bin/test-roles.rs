
use log::warn;
use poise::serenity_prelude as serenity;

type Result<T> = anyhow::Result<T>;
type Context<'a> = poise::Context<'a, (), anyhow::Error>;

#[poise::command(slash_command)]
async fn test_role(ctx: Context<'_>, role: serenity::Role) -> Result<()>
{
    ctx.say(format!("Hi the role is {}", role)).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> 
{
    env_logger::init();

    let dotenvy_result = dotenvy::dotenv();
    if dotenvy_result.is_err()
    {
        warn!("Couldn't load .env file");
    }

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![test_role()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(())
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    Ok(client.unwrap().start().await?)
}
