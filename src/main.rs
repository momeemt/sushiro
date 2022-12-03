use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;

#[derive(Debug, Clone, Copy)]
enum MenuKind {
    LimitedTime, // 期間限定
    Nigiri,
    Gunkan,
    SideMenu,
    Drink,
    Desert,
}

#[derive(Debug)]
struct Menu {
    kind: MenuKind,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = get_reqwest().await?;
    if let Some(menus) = try_parse_html(&result) {
        for menu in menus {
            println!("{:?}", menu)
        }
    }
    Ok(())
}

async fn get_reqwest() -> Result<String, Box<dyn std::error::Error>> {
    let body = reqwest::get("https://www.akindo-sushiro.co.jp/menu/").await?.text().await?;
    Ok(body)
}

fn parse_sushi_category(element: &ElementRef) -> Option<String> {
    let category_selector = Selector::parse("h3 a").unwrap();
    Some(element.select(&category_selector).next()?.text().collect::<String>().lines().collect::<String>())
}

fn parse_sushi_name(element: &ElementRef) -> Option<String> {
    let category_selector = Selector::parse("span.ttl").unwrap();
    Some(element.select(&category_selector).next()?.text().collect::<String>().lines().collect::<String>())
}

fn to_menu_kind(category: &str) -> Option<MenuKind> {
    match category {
        "期間限定" => Some(MenuKind::LimitedTime),
        "にぎり" => Some(MenuKind::Nigiri),
        "軍艦・巻物" => Some(MenuKind::Gunkan),
        "サイドメニュー" => Some(MenuKind::SideMenu),
        "ドリンク" => Some(MenuKind::Drink),
        "デザート" => Some(MenuKind::Desert),
        _ => None
    }
}

fn try_parse_html(html: &str) -> Option<Vec<Menu>>{
    let document = Html::parse_document(html);
    let selector_str = ".sec-wrap .c_l-content section";
    let selector = Selector::parse(selector_str).unwrap();
    let mut result: Vec<Menu> = vec![];

    for element in document.select(&selector) {
        let kind = to_menu_kind(&parse_sushi_category(&element)?)?;

        let selector = Selector::parse("ul.item-list li a").unwrap();
        for item in element.select(&selector) {
            let name = parse_sushi_name(&item)?;
            result.push(Menu{
                kind,
                name,
            })
        }
    }

    return Some(result);
}