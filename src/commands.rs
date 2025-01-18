use serenity::futures::future::join_all;
use songbird::{input::YoutubeDl, TrackEvent};
use tracing::instrument;

use crate::{
    events::TrackErrorNotifier,
    get_songbird_manager,
    typekeys::{HttpKey, SongLengthKey, SongTitleKey, SongUrlKey},
    Context, Error,
};

/// Show this help menu
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is a music bot written in Rust",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// Play a song or search YouTube for a song
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "What to play"] url: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild().map(|g| g.id) else {
        ctx.say("This command is only supported in guilds.").await?;
        return Ok(());
    };


    if !url.starts_with("http") {
        ctx.say("Must provide a valid URL").await?;
        return Ok(());
    }

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = get_songbird_manager(ctx).await;

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        let src = YoutubeDl::new(http_client, url.clone());

        let handle = handler.enqueue_input(src.clone().into()).await;
        let mut typemap = handle.typemap().write().await;

        tracing::info!("Pulling song information");

        let mut aux_multiple = src.clone()
            .search(Some(1))
            .await
            .expect("Failed to get info about song.");
        if aux_multiple.len() == 0 {}
        let aux = aux_multiple.swap_remove(0);
        let title = aux.title.unwrap_or_else(|| "Unknown".to_owned());
        let track_length = aux.duration.unwrap();

         typemap.insert::<SongTitleKey>(title.clone());
         typemap.insert::<SongUrlKey>(url);
         typemap.insert::<SongLengthKey>(format!(
             "{:0>2}:{:0>2}",
             (track_length.as_secs() / 60) % 60,
             track_length.as_secs() % 60
         ));

    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}

/// Join a voice channel
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, channel_id) = {
        let Some(guild) = ctx.guild() else {
            ctx.say("This command is only supported in guilds.").await?;
            return Ok(());
        };

        let channel_id = guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);
        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Not in a voice channel").await?;
            return Ok(());
        }
    };

    let manager = get_songbird_manager(ctx).await;
    match manager.join(guild_id, connect_to).await {
        Ok(handler_lock) => {
            // Attach an event handler to see notifications of all track errors.
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        }
        Err(e) => {
            println!("Faield to join channel: {:?}", e);
            ctx.say("Failed to join channel.").await?;
            return Err(Box::new(e));
        }
    }

    ctx.reply("Joined channel").await?;

    Ok(())
}

/// Leave the current voice channel
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild().map(|g| g.id) else {
        ctx.say("This command is only supported in guilds.").await?;
        return Ok(());
    };

    let manager = get_songbird_manager(ctx).await;
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            ctx.say(format!("Failed: {:?}", e)).await?;
        }

        ctx.say("Left voice channel").await?;
    } else {
        ctx.say("Not in a voice channel").await?;
    }

    Ok(())
}

/// Show the current queue
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild().map(|g| g.id) else {
        ctx.say("This command is only supported in guilds.").await?;
        return Ok(());
    };

    let songbird = get_songbird_manager(ctx).await;
    let Some(driver_lock) = songbird.get(guild_id) else {
        ctx.say("Not in a voice channel, no queue to show.").await?;
        return Ok(());
    };
    let driver = driver_lock.lock().await;
    if driver.queue().is_empty() {
        ctx.say("Queue is empty.").await?;
        return Ok(());
    }
    let current_uuid = driver.queue().current().map(|h| h.uuid());
    let queue = driver.queue().current_queue();
    let lines = queue.into_iter().enumerate().map(|(i, handle)| async move {
        let typemap = handle.typemap().read().await;
        let name = typemap
            .get::<SongTitleKey>()
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
            .to_owned();
        let url = typemap
            .get::<SongUrlKey>()
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
            .to_owned();
        let track_length = typemap
            .get::<SongLengthKey>()
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
            .to_owned();

        let play_time_duration = handle.get_info().await.ok().unwrap().play_time;
        let play_time = format!(
            "{:0>2}:{:0>2}",
            (play_time_duration.as_secs() / 60) % 60,
            play_time_duration.as_secs() % 60
        );

        if Some(handle.uuid()) == current_uuid {
            format!(
                "{}. {} - {} (currently playing {}/{})",
                i + 1,
                name,
                url,
                play_time,
                track_length
            )
        } else {
            format!("{}. {} - {} - {}", i + 1, name, url, track_length)
        }
    });
    let output = join_all(lines).await.join("\n");
    ctx.say(format!("## Queue:\n```\n{}\n```", output)).await?;

    Ok(())
}

/// Skip over the current song
#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild().map(|g| g.id) else {
        ctx.say("This command is only supported in guilds.").await?;
        return Ok(());
    };

    let songbird = get_songbird_manager(ctx).await;
    let Some(driver_lock) = songbird.get(guild_id) else {
        ctx.say("No playing anything, can't skip.").await?;
        return Ok(());
    };
    let driver = driver_lock.lock().await;
    driver.queue().skip()?;
    ctx.say("Skipping to the next song.").await?;

    Ok(())
}

#[instrument]
#[poise::command(prefix_command, slash_command)]
pub async fn version(ctx: Context<'_>) -> Result<(), Error> {
    let version = option_env!("CARGO_PKG_VERSION");
    ctx.say(version.unwrap_or("unknown")).await?;
    Ok(())
}
