use std:: {
    env,
    sync::Arc
};

use reqwest::Client as HttpClient;

use serenity::{all::{GuildId}, prelude::GatewayIntents};
use songbird::SerenityInit;

mod config;
use config::{load_config, Config};

mod commands;

mod events;


mod typekeys;
use typekeys::HttpKey;


#[derive(Debug, Clone)]
struct Data {
    config: Config,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn get_songbird_manager(ctx: Context<'_>) -> Arc<songbird::Songbird> {
    songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone()
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(err = %error, "Error in command \"{}\": {:?}", ctx.command().name, error);
            if let Err(e) = ctx.say("Error running command, please contact Hroi.").await {
                tracing::error!("Failed to warn user of crashed command: {}", e);
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: \"{}\":", e);
            } else {
                tracing::error!("Unknown error in poise.");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let config = load_config();
    let config_clone = config.clone();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::help(),
            commands::join(),
            commands::leave(),
            commands::play(),
            commands::queue(),
            commands::skip(),
            commands::version()
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("=".to_owned()),
            edit_tracker: None,
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);

                if cfg!(debug_assertions) {
                    println!("Running with debug enabled");
                    poise::builtins::register_in_guild(ctx, &framework.options().commands, GuildId::new(config_clone.test_guild.parse::<u64>().unwrap())).await?;
                } else {
                    println!("Running in production");
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }

                Ok(Data { config })
            })
        })
        .options(options)
        .build();

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::client::Client::builder(&token, intents)
        .framework(framework)
        .event_handler(events::Handler)
        .type_map_insert::<HttpKey>(HttpClient::new())
        .register_songbird()
        .await;

    client.unwrap().start().await.unwrap()
}
