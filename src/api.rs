use std::collections::HashMap;

use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Client, ClientBuilder, RequestBuilder, StatusCode};

use crate::{
    err::{self, delete, get, post, validate}, nexus_joiner, request::{CategoryName, Endorsements, GameId, ModFile, ModFiles, ModId, TrackedModsRaw, Validate}, VERSION
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

    async fn get_api(
        &self,
        ver: &str,
        slug: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(nexus_joiner!(ver, slug))
            .headers(extra_headers.iter().map(|(k, v)|
                (HeaderName::from_bytes(k.as_bytes()).unwrap(),
                 HeaderValue::from_str(v).unwrap())
            ).collect())
            .send()
            .await
    }

    fn post_api(&self, ver: &str, slug: &str) -> RequestBuilder {
        self.client
            .post(nexus_joiner!(ver, slug))
    }

    fn delete_api(&self, ver: &str, slug: &str) -> RequestBuilder {
        self.client
            .delete(nexus_joiner!(ver, slug))
    }

    /// Validate API key and retrieve user details.
    pub async fn validate(&self) -> Result<Validate, validate::ValidateError> {
        let response = self.get_api(VERSION, "users/validate", &[]).await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(validate::ValidateError::SerdeJson),
            StatusCode::UNAUTHORIZED => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(validate::ValidateError::InvalidAPIKey(err))
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404 (401), and 422"),
        }
    }

    /// Get a list of mods the user has endorsed.
    pub async fn endorsements(&self) -> Result<Endorsements, validate::ValidateError> {
        let response = self
            .get_api(VERSION, "user/endorsements", &[])
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(validate::ValidateError::SerdeJson),
            StatusCode::UNAUTHORIZED => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(validate::ValidateError::InvalidAPIKey(err))
            }
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
            .get_api(VERSION, "user/tracked_mods", &[])
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(validate::ValidateError::SerdeJson),
            StatusCode::UNAUTHORIZED => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
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
            .post_api(VERSION, "user/tracked_mods")
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
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
            StatusCode::NOT_FOUND => {
                let err: err::ModNotFoundError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
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
            .delete_api(VERSION, "user/tracked_mods")
            .query(&[("domain_name", game)])
            .form(&HashMap::from([("mod_id", id)]))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::NOT_FOUND => {
                let err: err::UntrackedOrInvalidMod =
                    serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
            _ => unreachable!("The only two documented return codes are 200 and 404"),
        }
    }

    /// Get a list of all games tracked by NexusMods.
    pub async fn games(&self) -> Result<Vec<GameId>, get::GameModError> {
        let response = self
            .get_api(VERSION, "games", &[])
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(get::GameModError::SerdeJson),
            StatusCode::NOT_FOUND => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
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
            .get_api(VERSION, &format!("games/{game}"), &[])
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(get::GameModError::SerdeJson),
            StatusCode::NOT_FOUND => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }

    /// Based on a game and a [`ModId`], get data about the download files the mod provides.
    pub async fn mod_files<S: Into<ModId>>(&self, game: &str, mod_id: S, category: Option<CategoryName>) -> Result<ModFiles, get::GameModError> {
        let mod_id = mod_id.into();
        let response = self
            .get_api(VERSION, &format!("games/{game}/mods/{mod_id}/files"), &category.iter().map(|c| ("category", c.to_header_str())).collect::<Vec<_>>())
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(get::GameModError::SerdeJson),
            StatusCode::NOT_FOUND => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }

    pub async fn mod_file<S: Into<ModId>>(&self, game: &str, mod_id: S, file_id: u64) -> Result<ModFile, get::GameModError> {
        let mod_id = mod_id.into();
        let response = self
            .get_api(VERSION, &format!("games/{game}/mods/{mod_id}/files/{file_id}"), &[])
            .await?;

        match response.status() {
            StatusCode::OK => serde_json::from_str(&response.text().await?)
                .map_err(get::GameModError::SerdeJson),
            StatusCode::NOT_FOUND => {
                let err: err::InvalidAPIKeyError = serde_json::from_str(&response.text().await?)?;
                Err(err.into())
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                unimplemented!(
                    "I have not yet encountered this return code but it is listed as a valid return code"
                );
            }
            _ => unreachable!("The only three documented return codes are 200, 404, and 422"),
        }
    }
}
