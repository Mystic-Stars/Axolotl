//! Minecraft official news from the Mojang launcher content API.
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::State;
use crate::util::fetch::fetch_json;

const LAUNCHER_CONTENT_BASE: &str = "https://launchercontent.mojang.com";
const MINECRAFT_SITE_BASE: &str = "https://www.minecraft.net";
const JAVA_NEWS_CATEGORY: &str = "Minecraft: Java Edition";

#[derive(Deserialize)]
struct NewsFile {
    entries: Vec<NewsEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewsEntry {
    title: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    news_page_image: Option<NewsImage>,
    #[serde(default)]
    play_page_image: Option<NewsImage>,
    #[serde(default)]
    read_more_link: Option<String>,
}

#[derive(Deserialize)]
struct NewsImage {
    url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MinecraftNewsItem {
    pub title: String,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub date: Option<String>,
    pub image_url: Option<String>,
    pub read_more_url: String,
}

pub async fn get_minecraft_news(
    limit: usize,
) -> crate::Result<Vec<MinecraftNewsItem>> {
    let state = State::get().await?;
    let news = fetch_json::<NewsFile>(
        Method::GET,
        &format!("{LAUNCHER_CONTENT_BASE}/v2/news.json"),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;

    let mut items: Vec<MinecraftNewsItem> = news
        .entries
        .into_iter()
        .filter(|entry| entry.category.as_deref() == Some(JAVA_NEWS_CATEGORY))
        .filter_map(|entry| {
            let read_more_url = article_url(entry.read_more_link.as_deref()?)?;
            Some(MinecraftNewsItem {
                title: entry.title,
                category: entry.category,
                tag: entry.tag,
                date: entry.date,
                image_url: entry
                    .news_page_image
                    .or(entry.play_page_image)
                    .map(|image| absolute_content_url(&image.url)),
                read_more_url,
            })
        })
        .collect();

    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(limit);
    Ok(items)
}

fn article_url(url: &str) -> Option<String> {
    let url = url.trim();
    let parsed = if url.starts_with('/') {
        url::Url::parse(MINECRAFT_SITE_BASE).ok()?.join(url).ok()?
    } else {
        url::Url::parse(url).ok()?
    };

    if matches!(parsed.scheme(), "http" | "https")
        && parsed.host_str().is_some()
    {
        Some(parsed.to_string())
    } else {
        None
    }
}

fn absolute_content_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{LAUNCHER_CONTENT_BASE}{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::article_url;

    #[test]
    fn normalizes_article_urls() {
        assert_eq!(
            article_url("/article/example"),
            Some("https://www.minecraft.net/article/example".to_string())
        );
        assert_eq!(
            article_url("https://www.minecraft.net/article/example"),
            Some("https://www.minecraft.net/article/example".to_string())
        );
        assert_eq!(article_url("javascript:alert(1)"), None);
        assert_eq!(article_url("https://"), None);
        assert_eq!(article_url("  "), None);
    }
}
