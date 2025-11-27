use std::{collections::HashMap, fmt::Display};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ModEntry {
    pub mod_id: ModId,
    pub domain_name: String,
}

/// A mod ID is a thin wrapper for a `u64`, but everywhere that you see [`ModId`], you can assume
/// that it is a valid NexusMods mod ID; it should always be valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModId {
    id: u64,
}

impl ModId {
    /// Get the underlying `u64`.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Secret teehee. Use this internally when you verify that a given u64 actually is a valid
    /// [`ModId`].
    pub(crate) const fn from_u64(id: u64) -> Self {
        Self { id }
    }
}

impl Display for ModId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl From<ModEntry> for ModId {
    fn from(value: ModEntry) -> Self {
        value.mod_id
    }
}

/// You may find this to be very tedious to work with. Consider [`TrackedMods`] instead.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrackedModsRaw {
    mods: Vec<ModEntry>,
}

impl TrackedModsRaw {
    /// Convert the faithful representation retrieved from NexusMods into a more rustic and
    /// idiomatic variant.
    pub fn into_mods(self) -> TrackedMods {
        let mut mods: HashMap<String, Vec<ModId>> = HashMap::with_capacity(self.mods.len());
        for entry in self.mods {
            mods.entry(entry.domain_name)
                .or_default()
                .push(entry.mod_id);
        }
        TrackedMods { mods }
    }
}

#[derive(Debug)]
pub struct TrackedMods {
    mods: HashMap<String, Vec<ModId>>,
}

impl TrackedMods {
    /// Get a list of [`ModId`]s from a game name.
    pub fn from_game(&self, name: &str) -> Option<&[ModId]> {
        self.mods.get(name).map(|v| &**v)
    }
}
