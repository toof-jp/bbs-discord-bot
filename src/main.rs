use std::sync::Arc;

use bbs_discord_bot::{Board, Handler, HandlerData};
use serenity::prelude::*;
use tokio::sync::Mutex;
use shuttle_runtime::SecretStore;

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_serenity::ShuttleSerenity {
    let user_session = secrets.get("USER_SESSION").expect("USER_SESSION must be set");
    let discord_token = secrets.get("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    let board = Board::new("https://ch.nicovideo.jp/unkchanel/bbs", "ch2598430");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler_data = Arc::new(Mutex::new(HandlerData {
        user_session,
        board,
    }));

    let handler = Handler {
        data: Arc::clone(&handler_data),
    };

    let client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    Ok(client.into())
}
