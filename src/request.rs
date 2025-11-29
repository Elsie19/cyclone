use std::{collections::HashMap, fmt::Display, ops::Deref, path::PathBuf, time::Duration};

use reqwest::Url;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use time::{OffsetDateTime, UtcDateTime};

#[macro_export]
macro_rules! nexus_joiner {
    ($ver:expr, $components:expr) => {{
        let mut url = reqwest::Url::parse("https://api.nexusmods.com")
            .expect("Could not parse URL (very fatal!)")
            .join(&format!("{}/", $ver))
            .expect("Could not join version!");
        let mut it = $components.into_iter().peekable();
        while let Some(comp) = it.next() {
            if it.peek().is_none() {
                url = url
                    .join(&format!("{}.json", comp))
                    .expect("Could not join {comp}");
            } else {
                url = url
                    .join(&format!("{}/", comp))
                    .expect("Could not join {comp}");
            }
        }
        url
    }};
}

#[derive(Clone, Copy)]
pub enum Limited {
    Hourly,
    Daily,
}

#[derive(Clone, Copy)]
pub struct RateLimiting {
    // Limited to 2,500 requests per 24 hours.
    pub(crate) hourly_limit: u16,
    pub(crate) hourly_remaining: u16,
    pub(crate) hourly_reset: OffsetDateTime,

    pub(crate) daily_limit: u16,
    pub(crate) daily_remaining: u16,
    pub(crate) daily_reset: OffsetDateTime,
}

impl RateLimiting {
    pub const fn limit(&self, limit: Limited) -> u16 {
        match limit {
            Limited::Hourly => self.hourly_limit,
            Limited::Daily => self.daily_limit,
        }
    }

    pub const fn remaining(&self, limit: Limited) -> u16 {
        match limit {
            Limited::Hourly => self.hourly_remaining,
            Limited::Daily => self.daily_remaining,
        }
    }

    pub const fn reset(&self, limit: Limited) -> UtcDateTime {
        match limit {
            Limited::Hourly => self.hourly_reset.to_utc(),
            Limited::Daily => self.daily_reset.to_utc(),
        }
    }
}

/// Validation object for a given user.
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
    /// Is the user a premium user?
    pub const fn is_premium(&self) -> bool {
        // I think?
        self.is_premium_q && self.is_premium
    }

    /// Is the user a supporter?
    ///
    /// In order for this to be `true`, the user must've bought premium at any point in time, even
    /// if they currently do not have it.
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

    /// URL to the user's NexusMods' avatar.
    ///
    /// # Warning
    /// This is *not* the path to the user's home page!
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

/// A checked and verified-to-exist mod ID.
///
/// A thin wrapper for a `u64`, but everywhere that you see [`ModId`], you can assume
/// that it is a valid mod ID, as opposed to a random number which may or may not exist.
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
}

impl From<TrackedModsRaw> for TrackedMods {
    fn from(value: TrackedModsRaw) -> Self {
        let mut mods: HashMap<String, Vec<ModId>> = HashMap::with_capacity(value.mods.len());
        for entry in value.mods {
            mods.entry(entry.domain_name)
                .or_default()
                .push(entry.mod_id);
        }
        Self { mods }
    }
}

/// A collection of game names and tracked mod IDs.
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

impl IntoIterator for TrackedMods {
    type Item = (String, Vec<ModId>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Vec<ModId>>;

    fn into_iter(self) -> Self::IntoIter {
        self.mods.into_iter()
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

    pub const fn date(&self) -> UtcDateTime {
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

    pub const fn approved_date(&self) -> UtcDateTime {
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

        impl Visitor<'_> for CategoryVisitor {
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

    pub const fn uploaded_at(&self) -> UtcDateTime {
        self.uploaded_timestamp.to_utc()
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
    pub(crate) const fn to_header_str(self) -> &'static str {
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

    pub const fn uploaded_at(&self) -> UtcDateTime {
        self.uploaded_timestamp.to_utc()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewFileRoot {
    children: Vec<PreviewFileChildren>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PreviewFileChildren {
    #[serde(rename = "directory")]
    Directory {
        path: String,
        name: String,
        children: Vec<PreviewFileChildren>,
    },
    #[serde(rename = "file")]
    File {
        path: String,
        name: String,
        size: String,
    },
}

impl PreviewFileChildren {
    pub fn into_pathbuf(self) -> PathBuf {
        match self {
            Self::File { path, .. } | Self::Directory { path, .. } => PathBuf::from(path),
        }
    }
}

impl PreviewFileRoot {
    /// Get all the files in the preview.
    pub fn files(&self) -> Vec<&PreviewFileChildren> {
        fn gather<'a>(node: &'a PreviewFileChildren, out: &mut Vec<&'a PreviewFileChildren>) {
            match node {
                PreviewFileChildren::Directory { children, .. } => {
                    for child in children {
                        gather(child, out);
                    }
                }
                PreviewFileChildren::File { .. } => {
                    out.push(node);
                }
            }
        }

        let mut out = vec![];

        for child in &self.children {
            gather(child, &mut out);
        }

        out
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModUpdated {
    mod_id: ModId,
    #[serde(with = "time::serde::timestamp")]
    latest_file_update: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    latest_mod_activity: OffsetDateTime,
}

impl ModUpdated {
    pub const fn id(&self) -> ModId {
        self.mod_id
    }

    pub const fn last_updated(&self) -> UtcDateTime {
        self.latest_file_update.to_utc()
    }

    pub const fn last_activity(&self) -> UtcDateTime {
        self.latest_mod_activity.to_utc()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimePeriod {
    Day,
    Week,
    Month,
}

impl TimePeriod {
    pub(crate) const fn as_str(&self) -> &str {
        match self {
            Self::Day => "1d",
            Self::Week => "1w",
            Self::Month => "1m",
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Duration> for TimePeriod {
    fn into(self) -> Duration {
        match self {
            Self::Day => Duration::from_hours(24),
            Self::Week => Duration::from_hours(24 * 7),
            Self::Month => Duration::from_hours(24 * 7 * 31),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Changelog {
    logs: HashMap<String, Vec<String>>,
}

impl Deref for Changelog {
    type Target = HashMap<String, Vec<String>>;

    fn deref(&self) -> &Self::Target {
        &self.logs
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameMod {
    name: String,
    summary: String,
    description: String,
    picture_url: Url,
    mod_downloads: u64,
    mod_unique_downloads: u64,
    uid: u64,
    game_id: u64,
    allow_rating: bool,
    domain_name: String,
    category_id: u64,
    version: String,
    endorsement_count: u64,
    #[serde(with = "time::serde::timestamp")]
    created_timestamp: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    created_time: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    updated_timestamp: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_time: OffsetDateTime,
    author: String,
    uploaded_by: String,
    uploaded_users_profile_url: Url,
    contains_adult_content: bool,
    // TODO: Make this an enum probably
    status: String,
    available: bool,
    #[serde(skip)]
    user: (),
    endorsement: EndorsementInfo,
}

impl GameMod {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub const fn mod_picture(&self) -> &Url {
        &self.picture_url
    }

    pub const fn unique_downloads(&self) -> u64 {
        self.mod_unique_downloads
    }

    pub const fn uid(&self) -> u64 {
        self.uid
    }

    pub const fn game_id(&self) -> u64 {
        self.game_id
    }

    pub const fn allow_rating(&self) -> bool {
        self.allow_rating
    }

    pub fn domain_name(&self) -> &str {
        &self.domain_name
    }

    pub const fn category_id(&self) -> u64 {
        self.category_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub const fn endorsements(&self) -> u64 {
        self.endorsement_count
    }

    pub const fn created_at(&self) -> UtcDateTime {
        self.created_timestamp.to_utc()
    }

    pub const fn updated_at(&self) -> UtcDateTime {
        self.updated_timestamp.to_utc()
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn uploaded_by(&self) -> &str {
        &self.uploaded_by
    }

    pub const fn uploaded_by_profile_url(&self) -> &Url {
        &self.uploaded_users_profile_url
    }

    pub const fn adult_content(&self) -> bool {
        self.contains_adult_content
    }

    pub const fn available(&self) -> bool {
        self.available
    }

    pub const fn endorsement(&self) -> &EndorsementInfo {
        &self.endorsement
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndorsementInfo {
    endorse_status: HasEndorsed,
    #[serde(serialize_with = "ts::serialize")]
    #[serde(deserialize_with = "ts::deserialize")]
    timestamp: Option<OffsetDateTime>,
    version: Option<String>,
}

impl EndorsementInfo {
    pub const fn status(&self) -> HasEndorsed {
        self.endorse_status
    }

    pub const fn has_endorsed(&self) -> bool {
        matches!(self.endorse_status, HasEndorsed::Endorsed)
    }

    pub const fn endorsed_at(&self) -> Option<OffsetDateTime> {
        self.timestamp
    }

    pub fn endorsed_version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HasEndorsed {
    Endorsed,
    Undecided,
}

mod ts {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(value: &Option<OffsetDateTime>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => s.serialize_i64(v.unix_timestamp()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<i64>::deserialize(d)?;
        Ok(opt.map(|secs| OffsetDateTime::from_unix_timestamp(secs).unwrap()))
    }
}
