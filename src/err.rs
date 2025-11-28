use std::fmt::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub struct InvalidAPIKeyError {
    pub message: String,
}

impl Display for InvalidAPIKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub struct ModNotFoundError {
    pub message: String,
}

impl Display for ModNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub struct UntrackedOrInvalidMod {
    pub message: String,
}

impl Display for UntrackedOrInvalidMod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub struct InvalidGame {
    pub code: u64,
    pub message: String,
}

impl Display for InvalidGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub mod validate {
    use thiserror::Error;

    use crate::err::InvalidAPIKeyError;

    #[derive(Debug, Error)]
    pub enum ValidateError {
        #[error(transparent)]
        Reqwest(#[from] reqwest::Error),
        #[error(transparent)]
        SerdeJson(#[from] serde_json::Error),
        #[error(transparent)]
        InvalidAPIKey(#[from] InvalidAPIKeyError),
    }
}

pub mod post {
    use thiserror::Error;

    use crate::{
        err::{InvalidAPIKeyError, ModNotFoundError},
        request::ModId,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PostModStatus {
        /// User successfully tracked a mod.
        SuccessfullyTracked(ModId),
        /// User is already tracking a mod.
        AlreadyTracking(ModId),
    }

    #[derive(Debug, Error)]
    pub enum TrackModError {
        #[error(transparent)]
        Reqwest(#[from] reqwest::Error),
        #[error(transparent)]
        SerdeJson(#[from] serde_json::Error),
        #[error(transparent)]
        InvalidAPIKey(#[from] InvalidAPIKeyError),
        #[error(transparent)]
        ModNotFound(#[from] ModNotFoundError),
    }
}

pub mod get {
    use thiserror::Error;

    use crate::err::{InvalidAPIKeyError, InvalidGame};

    #[derive(Debug, Error)]
    pub enum GameModError {
        #[error(transparent)]
        Reqwest(#[from] reqwest::Error),
        #[error(transparent)]
        SerdeJson(#[from] serde_json::Error),
        #[error(transparent)]
        InvalidAPIKey(#[from] InvalidAPIKeyError),
        #[error(transparent)]
        InvalidGameID(#[from] InvalidGame),
    }
}

pub mod delete {
    use thiserror::Error;

    use crate::err::{InvalidAPIKeyError, UntrackedOrInvalidMod};

    #[derive(Debug, Error)]
    pub enum DeleteModError {
        #[error(transparent)]
        Reqwest(#[from] reqwest::Error),
        #[error(transparent)]
        SerdeJson(#[from] serde_json::Error),
        #[error(transparent)]
        InvalidAPIKey(#[from] InvalidAPIKeyError),
        #[error(transparent)]
        UntrackedOrInvalid(#[from] UntrackedOrInvalidMod),
    }
}
