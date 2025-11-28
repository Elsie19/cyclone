use std::collections::HashMap;

use reqwest::{
    Client, ClientBuilder, Method, RequestBuilder, StatusCode,
    header::{HeaderMap, HeaderValue},
};

use crate::{
    VERSION,
    err::{self, delete, get, post, validate},
    nexus_joiner,
    request::{
        CategoryName, Endorsements, GameId, ModFile, ModFiles, ModId, ModUpdated, TimePeriod,
        TrackedModsRaw, Validate,
    },
};

/// Root level API handler.
pub struct Api {
    key: String,
    client: Client,
}

impl Api {
    /// Create a new wrapper with a [personal API key](https://next.nexusmods.com/settings/api-keys).
    pub fn new<S: Into<String>>(key: S) -> Self {
        let key = key.into();
        let client = ClientBuilder::new().default_headers({
            let mut h = HeaderMap::new();
            h.insert("apikey", key.parse().unwrap());
            h.insert("accept", HeaderValue::from_static("application/json"));
            h
        });
        Self {
            key,
            client: client.build().expect("oops"),
        }
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    fn build(
        &self,
        method: Method,
        ver: &str,
        slugs: &[&str],
        params: &[(&'static str, &str)],
    ) -> RequestBuilder {
        self.client
            .request(method, nexus_joiner!(ver, slugs))
            .query(params)
    }
}

/// User related methods.
///
/// # Status
///
/// - [x] `GET`    [`v1/users/validate`](`Api::validate`)
/// - [x] `GET`    [`v1/user/tracked_mods`](`Api::tracked_mods`)
/// - [x] `POST`   [`v1/user/tracked_mods`](`Api::track_mod`)
/// - [x] `DELETE` [`v1/user/tracked_mods`](`Api::untrack_mod`)
/// - [x] `GET`    [`v1/user/endorsements`](`Api::endorsements`)
impl Api {
    /// Validate API key and retrieve user details.
    pub async fn validate(&self) -> Result<Validate, validate::ValidateError> {
        let response = self
            .build(Method::GET, VERSION, &["users", "validate"], &[])
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response
                .json()
                .await
                .map_err(validate::ValidateError::Reqwest),
            StatusCode::UNAUTHORIZED => Err(validate::ValidateError::InvalidAPIKey(
                response.json().await?,
            )),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404 (401), and 422"),
        }
    }

    /// Get a list of the user's tracked mods.
    ///
    /// # Notes
    /// Consider converting to [`TrackedMods`](`crate::request::TrackedMods`) with
    /// [`crate::request::TrackedModsRaw::into_mods`].
    pub async fn tracked_mods(&self) -> Result<TrackedModsRaw, validate::ValidateError> {
        let response = self
            .build(Method::GET, VERSION, &["user", "tracked_mods"], &[])
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response
                .json()
                .await
                .map_err(validate::ValidateError::Reqwest),
            StatusCode::UNAUTHORIZED => Err(validate::ValidateError::InvalidAPIKey(
                response.json().await?,
            )),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404 (401), and 422"),
        }
    }

    /// Track a mod based on a `u64` mod ID.
    pub async fn track_mod<T: Into<u64>>(
        &self,
        game: &str,
        id: T,
    ) -> Result<post::PostModStatus, post::TrackModError> {
        let id = id.into();
        let response = self
            .build(Method::POST, VERSION, &["user", "tracked_mods"], &[])
            .query(&[("domain_name", game)])
            .form(&HashMap::from([("mod_id", id)]))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(post::PostModStatus::AlreadyTracking(ModId::from_u64(id))),
            StatusCode::CREATED => Ok(post::PostModStatus::SuccessfullyTracked(ModId::from_u64(
                id,
            ))),
            StatusCode::UNAUTHORIZED => {
                Err(response.json::<err::InvalidAPIKeyError>().await?.into())
            }
            StatusCode::NOT_FOUND => Err(response.json::<err::ModNotFoundError>().await?.into()),
            _ => unreachable!("The only four documented return codes are 200, 201, 404, and 401"),
        }
    }

    /// Untrack a mod.
    ///
    /// # Notes
    /// This function takes in a [`ModId`], not a `u64` because it is assumed that (unlike
    /// [`Api::track_mod`]) the caller knows of a valid mod ID.
    pub async fn untrack_mod<T: Into<ModId>>(
        &self,
        game: &str,
        id: T,
    ) -> Result<(), delete::DeleteModError> {
        let id = id.into();
        let response = self
            .build(Method::DELETE, VERSION, &["user", "tracked_mods"], &[])
            .query(&[("domain_name", game)])
            .form(&HashMap::from([("mod_id", id)]))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::NOT_FOUND => {
                Err(response.json::<err::UntrackedOrInvalidMod>().await?.into())
            }
            _ => unreachable!("The only two documented return codes are 200 and 404"),
        }
    }

    /// Get a list of mods the user has endorsed.
    pub async fn endorsements(&self) -> Result<Endorsements, validate::ValidateError> {
        let response = self
            .build(Method::GET, VERSION, &["user", "endorsements"], &[])
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response
                .json()
                .await
                .map_err(validate::ValidateError::Reqwest),
            StatusCode::UNAUTHORIZED => Err(validate::ValidateError::InvalidAPIKey(
                response.json().await?,
            )),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404 (401), and 422"),
        }
    }
}

/// Mod related methods.
///
/// - [x] `GET`  [`v1/games/{game_domain_name}/mods/updated`](`Api::updated_during`)
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/{mod_id}/changelogs`
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/latest_added`
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/latest_updated`
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/trending`
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/{id}`
/// - [ ] `GET`  `v1/games/{game_domain_name}/mods/md5_search/{md5_hash}`
/// - [ ] `POST` `v1/games/{game_domain_name}/mods/{id}/endorse`
/// - [ ] `POST` `v1/games/{game_domain_name}/mods/{id}/abstain`
impl Api {
    /// Get a list of mods updated within a timeframe.
    pub async fn updated_during(
        &self,
        game: &str,
        time: TimePeriod,
    ) -> Result<Vec<ModUpdated>, get::GameModError> {
        let response = self
            .build(
                Method::GET,
                VERSION,
                &["games", game, "mods", "updated"],
                &[("period", time.as_str())],
            )
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(get::GameModError::Reqwest),
            StatusCode::NOT_FOUND => Err(response.json::<err::InvalidAPIKeyError>().await?.into()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }
}

/// Game related methods.
///
/// - [x] `GET` [`v1/games`](`Api::games`)
/// - [x] `GET` [`v1/games/{game_domain_name}`](`Api::game`)
impl Api {
    /// Get a list of all games tracked by NexusMods.
    pub async fn games(&self) -> Result<Vec<GameId>, get::GameModError> {
        let response = self
            .build(Method::GET, VERSION, &["games"], &[])
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(get::GameModError::Reqwest),
            StatusCode::NOT_FOUND => Err(response.json::<err::InvalidAPIKeyError>().await?.into()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }

    /// Get information about a single game.
    pub async fn game(&self, game: &str) -> Result<GameId, get::GameModError> {
        let response = self
            .build(Method::GET, VERSION, &["games", game], &[])
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(get::GameModError::Reqwest),
            StatusCode::NOT_FOUND => Err(response.json::<err::InvalidAPIKeyError>().await?.into()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }
}

/// Mod file related methods.
///
/// - [x] `GET` [`v1/games/{game_domain_name}/mods/{mod_id}/files`](`Api::mod_files`)
/// - [x] `GET` [`v1/games/{game_domain_name}/mods/{mod_id}/files/{file_id}`](`Api::mod_file`)
/// - [ ] `GET` `v1/games/{game_domain_name}/mods/{mod_id}/files/{id}/download_link`
impl Api {
    /// Based on a game and a [`ModId`], get data about the download files the mod provides.
    pub async fn mod_files<S: Into<ModId>>(
        &self,
        game: &str,
        mod_id: S,
        category: Option<CategoryName>,
    ) -> Result<ModFiles, get::GameModError> {
        let mod_id = mod_id.into();
        let response = self
            .build(
                Method::GET,
                VERSION,
                &["games", game, "mods", mod_id.to_string().as_str(), "files"],
                &category
                    .iter()
                    .map(|c| ("category", c.to_header_str()))
                    .collect::<Vec<_>>(),
            )
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(get::GameModError::Reqwest),
            StatusCode::NOT_FOUND => Err(response.json::<err::InvalidAPIKeyError>().await?.into()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }

    pub async fn mod_file<S: Into<ModId>>(
        &self,
        game: &str,
        mod_id: S,
        file_id: u64,
    ) -> Result<ModFile, get::GameModError> {
        let mod_id = mod_id.into();
        let response = self
            .build(
                Method::GET,
                VERSION,
                &[
                    "games",
                    game,
                    "mods",
                    mod_id.to_string().as_str(),
                    "files",
                    file_id.to_string().as_str(),
                ],
                &[],
            )
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => response.json().await.map_err(get::GameModError::Reqwest),
            StatusCode::NOT_FOUND => Err(response.json::<err::InvalidAPIKeyError>().await?.into()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }
}
