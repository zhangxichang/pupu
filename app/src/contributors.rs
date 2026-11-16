use serde::Deserialize;

use crate::{Contributor, ContributorStatus, Viewport};

const API_ROOT: &str = "https://api.github.com";
const REPO: &str = "zhangxichang/pupu";

#[derive(Deserialize)]
struct ApiContributor {
    login: String,
    avatar_url: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct ApiUser {
    name: Option<String>,
    bio: Option<String>,
}

struct ContributorData {
    name: String,
    login: String,
    bio: String,
    avatar: Vec<u8>,
}

pub fn start(viewport: slint::Weak<Viewport>) {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let _ = tokio::spawn(async move {
        apply(viewport, fetch().await);
    });

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    wasm_bindgen_futures::spawn_local(async move {
        apply(viewport, fetch().await);
    });
}

fn apply(viewport: slint::Weak<Viewport>, data: Option<Vec<ContributorData>>) {
    let status = if data.is_some() {
        ContributorStatus::Loaded
    } else {
        ContributorStatus::Failed
    };
    let data = data.unwrap_or_default();
    let _ = slint::invoke_from_event_loop(move || {
        let Some(viewport) = viewport.upgrade() else {
            return;
        };
        let contributors = data
            .into_iter()
            .map(|data| Contributor {
                avatar: decode_avatar(&data.avatar),
                name: data.name.into(),
                login: data.login.into(),
                bio: data.bio.into(),
            })
            .collect::<Vec<_>>();
        viewport.set_contributors(slint::ModelRc::from(std::rc::Rc::new(
            slint::VecModel::from(contributors),
        )));
        viewport.set_contributor_status(status);
    });
}

fn decode_avatar(bytes: &[u8]) -> slint::Image {
    image::load_from_memory(bytes)
        .map(|image| {
            let image = image.to_rgba8();
            let buffer = slint::SharedPixelBuffer::clone_from_slice(
                image.as_raw(),
                image.width(),
                image.height(),
            );
            slint::Image::from_rgba8(buffer)
        })
        .unwrap_or_default()
}

fn avatar_url(url: &str) -> String {
    if url.contains('?') {
        format!("{url}&s=64")
    } else {
        format!("{url}?s=64")
    }
}

fn contributor_data(
    contributor: ApiContributor,
    user: Option<ApiUser>,
    avatar: Vec<u8>,
) -> ContributorData {
    let (name, bio) = match user {
        Some(user) => (
            user.name
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| contributor.login.clone()),
            user.bio.unwrap_or_default(),
        ),
        None => (contributor.login.clone(), String::new()),
    };
    ContributorData {
        name,
        login: contributor.login,
        bio,
        avatar,
    }
}

async fn github_get(
    client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()
}

async fn fetch() -> Option<Vec<ContributorData>> {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let client = match reqwest::Client::builder()
        .user_agent("pupu")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            log::warn!("创建 HTTP 客户端失败：{err}");
            return None;
        }
    };

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let client = reqwest::Client::new();

    let url = format!("{API_ROOT}/repos/{REPO}/contributors?per_page=100");
    let mut contributors: Vec<ApiContributor> = match github_get(&client, &url).await {
        Ok(response) => match response.json().await {
            Ok(contributors) => contributors,
            Err(err) => {
                log::warn!("解析贡献者列表失败：{err}");
                return None;
            }
        },
        Err(err) => {
            log::warn!("获取贡献者列表失败：{err}");
            return None;
        }
    };
    contributors
        .retain(|contributor| contributor.kind != "Bot" && !contributor.login.ends_with("[bot]"));

    let mut data = Vec::with_capacity(contributors.len());
    for contributor in contributors {
        let user_url = format!("{API_ROOT}/users/{}", contributor.login);
        let user = match github_get(&client, &user_url).await {
            Ok(response) => response.json::<ApiUser>().await.ok(),
            Err(_) => None,
        };
        let avatar = match github_get(&client, &avatar_url(&contributor.avatar_url)).await {
            Ok(response) => response
                .bytes()
                .await
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        data.push(contributor_data(contributor, user, avatar));
    }
    Some(data)
}
