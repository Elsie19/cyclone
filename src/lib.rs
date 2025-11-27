//! A rust wrapper for the Nexus Mods API.

pub static VERSION: &str = "v1";

pub mod api;
pub mod err;
pub mod request;

pub use api::Api;
