use std::sync::Arc;

use anyhow::{anyhow, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serenity::{async_trait, model::channel::Message, prelude::*};
use tokio::sync::Mutex;

pub struct HandlerData {
    pub user_session: String,
    pub board: Board,
}

pub struct Handler {
    pub data: Arc<Mutex<HandlerData>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.mentions_me(&ctx).await.unwrap_or(false) {
            let data = self.data.lock().await;
            let user_session = &data.user_session;
            let mut board = data.board.clone();

            let from = msg.author.display_name();
            let mut content = remove_mentions(&msg.content).trim().to_string();
            content.push_str(&format!(
                "\n\nmessage_id: {}\nuser_id: {}",
                msg.id, msg.author.id,
            ));

            let response = match post_res(user_session, &mut board, from, &content).await {
                Ok(res) => res,
                Err(e) => e.to_string(),
            };

            if let Err(e) = msg.channel_id.say(&ctx.http, response).await {
                println!("Error sending message: {e:?}");
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

#[test]
fn test_remove_mentions() {
    assert_eq!(remove_mentions("<@123> foo"), " foo");
    assert_eq!(remove_mentions("<@123> <@456> foo"), "  foo");
}

async fn post_res(
    user_session: &str,
    board: &mut Board,
    from: &str,
    message: &str,
) -> Result<String> {
    let bbs_url = format!("https://dic.nicovideo.jp/b/c/{}/p", board.bbs_id);
    board.get_hash_key(user_session).await;
    let cookie_value = format!(
        "hash_key={};{}",
        board.hash_key.clone().unwrap(),
        user_session
    );

    let res = reqwest::Client::new()
        .post(bbs_url)
        .header(reqwest::header::COOKIE, cookie_value)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("FROM={}&MESSAGE={}&magic=dummy", from, message))
        .send()
        .await?;

    let html = res.text().await?;

    if html.contains("投稿間隔が短すぎです") {
        Err(anyhow!("投稿間隔が短すぎです。300秒待ってください。"))
    } else if html.contains("投稿内容が長すぎです") {
        Err(anyhow!("投稿内容が長すぎです。1024文字に収めてください。"))
    } else if html.contains("投稿内容に長すぎる行があります") {
        Err(anyhow!(
            "投稿内容に長すぎる行があります。1行は192文字に収めてください。"
        ))
    } else if html.contains("投稿を受け付けました") {
        Ok("投稿を受け付けました！".to_string())
    } else {
        Err(anyhow!("投稿に失敗しました。"))
    }
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
