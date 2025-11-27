use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcDateTime};

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
    user_id: usize,
    key: String,
    name: String,
    #[serde(alias = "is_premium?")]
    is_premium_q: bool,
    #[serde(alias = "is_supporter?")]
    is_supporter_q: bool,
    email: String,
    profile_url: String,
    is_premium: bool,
    is_supporter: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModEntry {
    mod_id: ModId,
    domain_name: String,
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

impl PartialEq<u64> for ModId {
    fn eq(&self, other: &u64) -> bool {
        self.id() == *other
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Endorsements {
    mods: Vec<Endorsement>,
}

impl IntoIterator for Endorsements {
    type Item = Endorsement;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.mods.into_iter()
    }
}

impl Endorsements {
    pub fn find<F>(&self, func: F) -> Option<&Endorsement>
    where
        F: Fn(&Endorsement) -> bool,
    {
        self.mods.iter().find(|e| func(e))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Endorsement {
    mod_id: ModId,
    domain_name: String,
    #[serde(with = "time::serde::iso8601")]
    date: OffsetDateTime,
    version: Option<String>,
    status: EndorseStatus,
}

impl Endorsement {
    pub const fn id(&self) -> ModId {
        self.mod_id
    }

    pub fn domain_name(&self) -> &str {
        &self.domain_name
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn date(&self) -> UtcDateTime {
        self.date.to_utc()
    }

    pub const fn endorsed_status(&self) -> EndorseStatus {
        self.status
    }

    pub const fn is_endorsed(&self) -> bool {
        matches!(self.status, EndorseStatus::Endorsed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndorseStatus {
    Endorsed,
    #[serde(untagged)]
    NotEndorsed,
}
