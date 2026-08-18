use std::env;

use reqwest::blocking::{Client, RequestBuilder};

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

    pub(crate) fn with_auth(&self, request: RequestBuilder, token: &str) -> RequestBuilder {
        request.bearer_auth(token)
    }

    pub(crate) fn with_optional_auth(
        &self,
        request: RequestBuilder,
        token: Option<&str>,
    ) -> RequestBuilder {
        match token {
            Some(token) => self.with_auth(request, token),
            None => request,
        }
    }
}
