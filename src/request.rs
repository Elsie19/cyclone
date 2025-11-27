use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! nexus_joiner {
    ($ver:expr, $slug:expr) => {
        reqwest::Url::parse("https://api.nexusmods.com")
            .expect("Could not parse URL")
            .join(&format!("{}/", $ver))
            .expect("Could not join version")
            .join(&format!("{}.json", $slug))
            .expect("Could not join slug")
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Validate {
    pub user_id: usize,
    pub key: String,
    pub name: String,
    #[serde(alias = "is_premium?")]
    pub is_premium_q: bool,
    #[serde(alias = "is_supporter?")]
    pub is_supporter_q: bool,
    pub email: String,
    pub profile_url: String,
    pub is_premium: bool,
    pub is_supporter: bool,
}
