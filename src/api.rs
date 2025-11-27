use std::collections::HashMap;

use reqwest::{Client, ClientBuilder, RequestBuilder, StatusCode, header::HeaderMap};

use crate::{
    err::{self, delete, post, validate}, nexus_joiner, request::{Endorsements, ModId, TrackedModsRaw, Validate}, VERSION
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
        let mut headers = HeaderMap::new();
        headers.insert("apikey", key.parse().unwrap());
        let client = ClientBuilder::new().default_headers(headers);
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
        key: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(nexus_joiner!(ver, slug))
            .header("accept", "application/json")
            .header("apikey", key)
            .send()
            .await
    }

    fn post_api(&self, ver: &str, slug: &str, key: &str) -> RequestBuilder {
        self.client
            .post(nexus_joiner!(ver, slug))
            .header("accept", "application/json")
            .header("apikey", key)
    }

    fn delete_api(&self, ver: &str, slug: &str, key: &str) -> RequestBuilder {
        self.client
            .delete(nexus_joiner!(ver, slug))
            .header("accept", "application/json")
            .header("apikey", key)
    }

    /// Validate API key and retrieve user details.
    pub async fn validate(&self) -> Result<Validate, validate::ValidateError> {
        let response = self.get_api(VERSION, "users/validate", self.key()).await?;

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

    pub async fn endorsements(&self) -> Result<Endorsements, validate::ValidateError> {
        let response = self
            .get_api(VERSION, "user/endorsements", self.key())
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
            .get_api(VERSION, "user/tracked_mods", self.key())
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
            .post_api(VERSION, "user/tracked_mods", self.key())
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
            .delete_api(VERSION, "user/tracked_mods", self.key())
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
}
