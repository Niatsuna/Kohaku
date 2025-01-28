/// Honkai: Star Rail specific format functions to format parsed data to database data
use regex::Regex;

use super::models::db::item::NewItem as DatabaseItem;
use super::models::parse::item::Item as ParsedItem;
use super::scrapers::item::{API_ENDPOINT, WIKI_URL};
use super::scrapers::BASE_URL;
use crate::error::KohakuError;

fn _remove_html_from_string(str: &str) -> String {
    let re = Regex::new(r#"<[^>]+>"#).unwrap();
    re.replace_all(str, "").trim().to_string()
}

fn _format_purpose(purpose: &i32) -> Option<Vec<String>> {
    let res = match purpose {
        1 => vec!["Character EXP Material"],
        2 => vec!["Character Ascension Material"],
        3 => vec!["Light Cone Ascension Material", "Trace Material"],
        4 => vec!["Trace Material"],
        5 => vec!["Light Cone EXP Material"],
        6 => vec!["Relic EXP Material"],
        7 => vec!["Character Ascension Material", "Trace Material"],
        8 | 9 => vec!["Warp Item"],
        11 | 12 | 13 => vec!["Currency"],
        _ => vec![],
    };

    if res.len() == 0 {
        None
    } else {
        Some(res.iter().map(|s| s.to_string()).collect())
    }
}

fn _format_rarity(rarity: &str) -> Option<i32> {
    match rarity {
        "Normal" => Some(1),
        "NotNormal" => Some(2),
        "Rare" => Some(3),
        "VeryRare" => Some(4),
        "SuperRare" => Some(5),
        _ => None,
    }
}

pub fn format_item(index: &str, data: ParsedItem) -> Result<DatabaseItem, KohakuError> {
    let name = data.name.clone();

    // Rarity: String -> Number
    let rarity = _format_rarity(&data.rarity);
    if rarity == None {
        return Err(KohakuError::CustomError(
            "Couldn't map rarity to a number".to_string(),
        ));
    }

    // PurposeType: Number -> String
    let types = _format_purpose(&data.purpose_type);
    if types == None {
        return Err(KohakuError::CustomError(
            "Couldn't map purpose types to a string".to_string(),
        ));
    }

    // Descriptions : Remove HTML Tags
    let description = match data.desc {
        Some(desc) => {
            let t = _remove_html_from_string(&desc);
            if t == "" {
                None
            } else {
                Some(t)
            }
        }
        None => None,
    };

    let description_bg = match data.bgdesc {
        Some(desc) => {
            let t = _remove_html_from_string(&desc);
            if t == "" {
                None
            } else {
                Some(t)
            }
        }
        None => None,
    };

    // Format
    let sources = data
        .source
        .iter()
        .map(|src| src.desc.clone())
        .collect::<Vec<String>>();

    let filename = data
        .icon_path
        .split('/')
        .last()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("");
    let icon_path = format!("itemfigures/{filename}.webp");

    let api_url = format!("{BASE_URL}/{API_ENDPOINT}/{index}.json");
    let wiki_url = format!("{WIKI_URL}/{index}");
    let img_url = format!("{BASE_URL}/UI/{icon_path}");

    // ---
    Ok(DatabaseItem {
        name: name,
        rarity: rarity.unwrap(),
        description: description,
        description_bg: description_bg,
        types: types.unwrap(),
        sources: sources,
        item_group: data.item_group,
        api_url: api_url,
        wiki_url: wiki_url,
        img_url: img_url,
    })
}
