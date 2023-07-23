use std::env;
use std::time::SystemTime;

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
};

use dotenv::dotenv;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::Client;

use anyhow::Result;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum MenuKind {
    LimitedTime, // 期間限定
    Nigiri,
    Gunkan,
    SideMenu,
    Drink,
    Desert,
}

#[derive(Debug, Serialize, Deserialize)]
struct Menu {
    kind: MenuKind,
    name: String,
}

#[group]
#[commands(roll)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("/"))
        .group(&GENERAL_GROUP);
    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_PRESENCES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let result = get_reqwest().await?;

    let menus = try_parse_html(&result).unwrap();
    write_file(menus).await?;

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}

async fn write_file(menus: Vec<Menu>) -> Result<()> {
    let serialized: String = serde_json::to_string(&menus)?;
    let mut file = File::create("menus.json").await?;
    file.write_all(serialized.as_bytes()).await?;
    Ok(())
}

async fn get_reqwest() -> Result<String> {
    let body = reqwest::get("https://www.akindo-sushiro.co.jp/menu/menu_detail/?s_id=528")
        .await?
        .text()
        .await?;
    Ok(body)
}

fn parse_sushi_category(element: &ElementRef) -> Option<String> {
    let category_selector = Selector::parse("h3 a").ok()?;
    Some(
        element
            .select(&category_selector)
            .next()?
            .text()
            .collect::<String>()
            .lines()
            .collect::<String>(),
    )
}

fn parse_sushi_name(element: &ElementRef) -> Option<String> {
    let category_selector = Selector::parse("span.ttl").ok()?;
    Some(
        element
            .select(&category_selector)
            .next()?
            .text()
            .collect::<String>()
            .lines()
            .collect::<String>(),
    )
}

fn to_menu_kind(category: &str) -> Option<MenuKind> {
    match category {
        "期間限定" => Some(MenuKind::LimitedTime),
        "にぎり" => Some(MenuKind::Nigiri),
        "軍艦・巻物" => Some(MenuKind::Gunkan),
        "サイドメニュー" => Some(MenuKind::SideMenu),
        "ドリンク" => Some(MenuKind::Drink),
        "デザート" => Some(MenuKind::Desert),
        _ => None,
    }
}

fn try_parse_html(html: &str) -> Option<Vec<Menu>> {
    let document = Html::parse_document(html);
    let selector_str = ".sec-wrap .c_l-content section";
    let selector = Selector::parse(selector_str).unwrap();
    let mut result: Vec<Menu> = vec![];

    for element in document.select(&selector) {
        let kind = to_menu_kind(&parse_sushi_category(&element)?)?;

        let selector = Selector::parse("ul.item-list li a").unwrap();
        for item in element.select(&selector) {
            let name = parse_sushi_name(&item)?;
            result.push(Menu { kind, name })
        }
    }

    Some(result)
}

#[command]
async fn roll(ctx: &Context, msg: &Message) -> CommandResult {
    let mut message = String::from("");
    let file = File::open("menus.json").await?;
    let mut f = BufReader::new(file);
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).await?;
    let deserialized: Vec<Menu> = serde_json::from_str(std::str::from_utf8(&buffer)?).unwrap();
    let guild = ctx.cache.guild(msg.guild_id.unwrap()).unwrap();
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Duration since UNIX_EPOCH failed");
    let mut rng = StdRng::seed_from_u64(d.as_secs());
    for (_, member) in guild.members {
        if !member.user.bot {
            let menu = deserialized.iter().choose_multiple(&mut rng, 1)[0];
            message += &format!("{}: {}\n", member.display_name(), menu.name);
        }
    }
    msg.reply(ctx, message).await?;
    Ok(())
}
