use std::sync::Arc;

use anyhow::Result;
use dotenv::dotenv;
use regex::Regex;
use scraper::{Html, Selector};
use serenity::async_trait;
use serenity::model::channel::Message;
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

struct HandlerData {
    user_session: String,
    board: Board,
}

struct Handler {
    data: Arc<Mutex<HandlerData>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.mentions_me(&ctx).await.unwrap_or(false) {
            let data = self.data.lock().await;
            let content = remove_mentions(&msg.content).trim().to_string();
            let user_session = &data.user_session;
            let mut board = data.board.clone();
            post_res(user_session, &mut board, "ななしのよっしん", &content)
                .await
                .expect("error post res");

            dbg!(msg.author);
            if let Err(why) = msg.channel_id.say(&ctx.http, "投稿しました").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, _: Context, ready: serenity::model::gateway::Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn remove_mentions(input: &str) -> String {
    let re = Regex::new(r"<@\d+>").unwrap();
    re.replace_all(input, "").into_owned()
}

async fn post_res(user_session: &str, board: &mut Board, from: &str, message: &str) -> Result<()> {
    let bbs_url = format!("https://dic.nicovideo.jp/b/c/{}/p", board.bbs_id);
    board.get_hash_key(user_session).await;
    dbg!(board.hash_key.clone().unwrap());
    let cookie_value = format!(
        "hash_key={};{}",
        board.hash_key.clone().unwrap(),
        user_session
    );
    dbg!(&cookie_value);

    let res = reqwest::Client::new()
        .post(bbs_url)
        .header(reqwest::header::COOKIE, cookie_value)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("FROM={}&MESSAGE={}&magic=dummy", from, message))
        .send()
        .await?;

    dbg!(res);

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Board {
    pub top_bbs_url: String,
    pub bbs_id: String,
    pub hash_key: Option<String>,
}

impl Board {
    pub fn new(top_bbs_url: &str, bbs_id: &str) -> Board {
        Board {
            top_bbs_url: top_bbs_url.to_string(),
            bbs_id: bbs_id.to_string(),
            hash_key: None,
        }
    }

    pub async fn get_hash_key(&mut self, user_session: &str) {
        let client = reqwest::ClientBuilder::new().build().unwrap(); // TODO client builder不要？
        let request = client
            .get(&self.top_bbs_url)
            .header(reqwest::header::COOKIE, user_session);
        let response = request.send().await.unwrap();

        dbg!(&response);

        self.hash_key = Some(Self::parse_top_bbs_html(&response.text().await.unwrap()));
    }

    fn parse_top_bbs_html(html: &str) -> String {
        let document = Html::parse_document(html);
        let iframe_selector = Selector::parse("#community-bbs").unwrap();

        let url_with_hash_key = document
            .select(&iframe_selector)
            .next()
            .unwrap()
            .value()
            .attr("src")
            .unwrap();
        let url_with_hash_key = reqwest::Url::parse(url_with_hash_key).unwrap();
        let hash_key = url_with_hash_key
            .query_pairs()
            .next()
            .unwrap()
            .1
            .to_string();

        hash_key
    }
}
