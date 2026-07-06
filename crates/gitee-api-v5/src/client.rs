use std::env;

use reqwest::blocking::Client;

use crate::utils::resolve_base_url;

pub struct GiteeClient {
    pub(crate) client: Client,
    pub(crate) base_url: String,
}

impl GiteeClient {
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            base_url: resolve_base_url(base_url.map(str::to_string)),
        }
    }

    pub fn from_env() -> Self {
        Self::new(env::var("GITEE_BASE_URL").ok().as_deref())
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
