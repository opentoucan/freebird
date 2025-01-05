use serenity::{
    async_trait,
    client::{ Context, EventHandler },
    model::{
        gateway::Ready,
        voice::VoiceState,
    }
};
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};

use crate::typekeys::SongUrlKey;

pub struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                let typemap = handle.typemap().read().await;
                let url = typemap
                    .get::<SongUrlKey>()
                    .map(|src| src.as_str())
                    .unwrap_or("Unknown");
                tracing::error!(?handle, ?state, "Track \"{}\" encountered an error.", url);
            }
        }

        None
    }
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        {
            let voice_state = match old {
                Some(old) => old,
                None => {
                    return;
                }
            };
            let channel_id = voice_state.channel_id;
            let channel_id = match channel_id {
                Some(channel_id) => channel_id,
                None => {
                    return;
                }
            };

            let guild_option = ctx.cache.guild(new.guild_id.unwrap());
            let guild = match guild_option {
                Some(guild_option) => guild_option,
                None => {
                    return;
                }
            };

            let user_count = guild
                .voice_states
                .values()
                .filter(|state| match state.channel_id {
                    Some(c) => c == channel_id,
                    None => false,
                })
                .count();

            if user_count > 1 {
                return;
            }
        }

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let _ = manager.remove(new.guild_id.unwrap()).await;
    }
}
