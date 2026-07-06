use serde::Deserialize;

#[derive(Deserialize)]
struct ApiErrorResponse {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

pub(crate) fn parse_api_error_message(response: reqwest::blocking::Response) -> Option<String> {
    let body = response.text().ok()?;
    if body.trim().is_empty() {
        return None;
    }

    if let Ok(payload) = serde_json::from_str::<ApiErrorResponse>(&body) {
        return payload
            .message
            .or(payload.error_description)
            .or(payload.error);
    }

    Some(body)
}

pub(crate) fn resolve_base_url(value: Option<String>) -> String {
    value
        .unwrap_or_else(|| "https://gitee.com/api".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_base_url;

    #[test]
    fn defaults_to_gitee_api_base_path() {
        assert_eq!(resolve_base_url(None), "https://gitee.com/api");
    }

    #[test]
    fn trims_trailing_slash_from_custom_base_url() {
        assert_eq!(
            resolve_base_url(Some("http://127.0.0.1:1234/".to_string())),
            "http://127.0.0.1:1234"
        );
    }
}
