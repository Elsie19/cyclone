use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use reqwest::Url;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
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
    profile_url: Url,
    is_premium: bool,
    is_supporter: bool,
}

impl Validate {
    pub const fn is_premium(&self) -> bool {
        // I think?
        self.is_premium_q && self.is_premium
    }

    pub const fn is_supporter(&self) -> bool {
        // I think?
        self.is_supporter_q && self.is_supporter
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &Url {
        &self.profile_url
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModEntry {
    mod_id: ModId,
    domain_name: String,
}

impl ModEntry {
    pub const fn id(&self) -> ModId {
        self.mod_id
    }

    pub fn domain_name(&self) -> &str {
        &self.domain_name
    }
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
    pub fn mods(&self) -> &[ModEntry] {
        &self.mods
    }

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
    pub fn get_game(&self, name: &str) -> Option<&[ModId]> {
        self.mods.get(name).map(|v| &**v)
    }

    /// Get all game names.
    pub fn games(&self) -> impl Iterator<Item = &str> {
        self.mods.keys().map(String::as_str)
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GameId {
    id: u64,
    name: String,
    forum_url: Url,
    nexusmods_url: Url,
    genre: String,
    file_count: u64,
    domain_name: String,
    #[serde(with = "time::serde::timestamp")]
    approved_date: OffsetDateTime,
    file_views: u64,
    authors: u64,
    file_endorsements: u64,
    mods: u64,
    categories: Vec<GameCategory>,
}

impl GameId {
    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn pretty_name(&self) -> &str {
        &self.name
    }

    pub fn forum(&self) -> &Url {
        &self.forum_url
    }

    pub fn page(&self) -> &Url {
        &self.nexusmods_url
    }

    pub fn genre(&self) -> &str {
        &self.genre
    }

    pub fn domain_name(&self) -> &str {
        &self.domain_name
    }

    pub fn approved_date(&self) -> UtcDateTime {
        self.approved_date.to_utc()
    }

    pub const fn file_views(&self) -> u64 {
        self.file_views
    }

    pub const fn authors(&self) -> u64 {
        self.authors
    }

    pub const fn endorsements(&self) -> u64 {
        self.file_endorsements
    }

    pub const fn mods(&self) -> u64 {
        self.mods
    }

    pub fn categories(&self) -> &[GameCategory] {
        &self.categories
    }

    /// Get the parent category for a given category.
    pub fn trace_parent_category(&self, category: &GameCategory) -> Option<&GameCategory> {
        let id = &category.parent_category;
        self.categories.iter().find(|cat| match id {
            Category::Category(n) => *n == cat.category_id,
            Category::None => false,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameCategory {
    category_id: u64,
    name: String,
    parent_category: Category,
}

#[derive(Debug)]
pub enum Category {
    Category(u64),
    None,
}

impl<'de> Deserialize<'de> for Category {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CategoryVisitor;

        impl<'de> Visitor<'de> for CategoryVisitor {
            type Value = Category;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a number or false")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Category::Category(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.is_negative() {
                    Err(de::Error::custom("negative number not allowed"))
                } else {
                    Ok(Category::Category(v as u64))
                }
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !v {
                    Ok(Category::None)
                } else {
                    Err(de::Error::custom("`true` not allowed"))
                }
            }
        }

        de.deserialize_any(CategoryVisitor)
    }
}

impl Serialize for Category {
    fn serialize<S>(&self, se: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Self::Category(n) => se.serialize_u64(n),
            Self::None => se.serialize_bool(false),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModFiles {
    files: Vec<ModFile>,
    file_updates: Vec<FileUpdate>,
}

impl ModFiles {
    pub fn iter_files(&self) -> impl Iterator<Item = &ModFile> {
        self.files.iter()
    }

    pub fn iter_updates(&self) -> impl Iterator<Item = &FileUpdate> {
        self.file_updates.iter()
    }

    pub fn into_iter_files(self) -> impl IntoIterator<Item = ModFile> {
        self.files.into_iter()
    }

    pub fn into_iter_updates(self) -> impl IntoIterator<Item = FileUpdate> {
        self.file_updates.into_iter()
    }

    /// Deduplicate entries based on a condition.
    ///
    /// Mostly useful for when you want to just get a single throwaway instance of [`ModFile`],
    /// likely for printing out something pertaining to the mod as a whole, rather than every file
    /// located inside it, such as a loop to print what names of mods the user endorses.
    pub fn dedup<F>(&self, same: F) -> Vec<ModFile>
    where
        F: Fn(&ModFile, &ModFile) -> bool,
    {
        let mut out = vec![];

        'outer: for x in &self.files {
            for y in &out {
                if same(x, y) {
                    continue 'outer;
                }
            }
            out.push(x.clone());
        }

        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    id: Vec<u64>,
    uid: u64,
    file_id: u64,
    name: String,
    version: String,
    category_id: u64,
    category_name: CategoryName,
    is_primary: bool,
    size: u64,
    file_name: String,
    #[serde(with = "time::serde::timestamp")]
    uploaded_timestamp: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    uploaded_time: OffsetDateTime,
    mod_version: String,
    external_virus_scan_url: Option<Url>,
    description: Option<String>,
    size_kb: u64,
    size_in_bytes: u64,
    changelog_html: Option<String>,
    content_preview_link: Url,
}

impl ModFile {
    pub fn ids(&self) -> &[u64] {
        &self.id
    }

    pub const fn uid(&self) -> u64 {
        self.uid
    }

    pub const fn file_id(&self) -> u64 {
        self.file_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub const fn category_id(&self) -> u64 {
        self.category_id
    }

    pub const fn category_name(&self) -> CategoryName {
        self.category_name
    }

    pub const fn is_primary(&self) -> bool {
        self.is_primary
    }

    /// Appears to be in kilobytes.
    pub const fn size(&self) -> u64 {
        self.size
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn uploaded_timestamp(&self) -> UtcDateTime {
        self.uploaded_timestamp.to_utc()
    }

    pub fn uploaded_timestamp_epoch(&self) -> i64 {
        self.uploaded_timestamp.unix_timestamp()
    }

    pub fn uploaded_time(&self) -> UtcDateTime {
        self.uploaded_time.to_utc()
    }

    pub fn mod_version(&self) -> &str {
        &self.mod_version
    }

    pub fn virus_scan_url(&self) -> Option<&Url> {
        self.external_virus_scan_url.as_ref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub const fn size_kb(&self) -> u64 {
        self.size_kb
    }

    pub const fn size_bytes(&self) -> u64 {
        self.size_in_bytes
    }

    pub fn changelog(&self) -> Option<&str> {
        self.changelog_html.as_deref()
    }

    pub fn content_preview(&self) -> &Url {
        &self.content_preview_link
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CategoryName {
    Main,
    Update,
    Optional,
    OldVersion,
    Miscellaneous,
    Archived,
}

impl CategoryName {
    pub(crate) const fn to_header_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Update => "update",
            Self::Optional => "optional",
            Self::OldVersion => "old_version",
            Self::Miscellaneous => "miscellaneous",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileUpdate {
    old_file_id: u64,
    new_file_id: u64,
    old_file_name: String,
    new_file_name: String,
    #[serde(with = "time::serde::timestamp")]
    uploaded_timestamp: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    uploaded_time: OffsetDateTime,
}

impl FileUpdate {
    /// Return the old and new ID.
    pub const fn ids(&self) -> (u64, u64) {
        (self.old_file_id, self.new_file_id)
    }

    /// Return the old and new names.
    pub fn names(&self) -> (&str, &str) {
        (&self.old_file_name, &self.new_file_name)
    }

    pub fn uploaded_timestamp(&self) -> UtcDateTime {
        self.uploaded_timestamp.to_utc()
    }

    pub fn uploaded_time(&self) -> UtcDateTime {
        self.uploaded_time.to_utc()
    }
}
