use std::sync::Arc;

use bbs_discord_bot::{Board, Handler, HandlerData};
use dotenv::dotenv;
use serenity::prelude::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let user_session = std::env::var("USER_SESSION").expect("USER_SESSION must be set");
    let discord_token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let board = Board::new("https://ch.nicovideo.jp/unkchanel/bbs", "ch2598430");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler_data = Arc::new(Mutex::new(HandlerData {
        user_session,
        board,
    }));

    let handler = Handler {
        data: Arc::clone(&handler_data),
    };

    let mut client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
