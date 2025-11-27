use reqwest::{Client, ClientBuilder, header::HeaderMap};

use crate::{err::GetOrParseError, nexus_joiner, request::Validate};

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

    pub async fn validate(&self) -> Result<Validate, GetOrParseError> {
        let response = self
            .client
            .get(nexus_joiner!("v1", "users/validate"))
            .send()
            .await?
            .text()
            .await?;

        serde_json::from_str(&response).map_err(GetOrParseError::SerdeJsonError)
    }
}
