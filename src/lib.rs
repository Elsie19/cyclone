#![doc(
    html_logo_url = "https://freepngimg.com/thumb/hurricane/31308-2-hurricane-clipart.png",
    html_favicon_url = "https://freepngimg.com/thumb/hurricane/31308-2-hurricane-clipart.png"
)]
//! A rust wrapper for the Nexus Mods API.

pub(crate) static VERSION: &str = "v1";

mod api;
pub mod err;
pub mod request;

pub use api::Api;
