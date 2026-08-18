use serde::Deserialize;

enum InvalidTokenCheck {
    BeforeNotFound,
    AfterNotFound,
}

enum NotFoundBehavior {
    MapToNotFound,
    KeepUnexpectedStatus,
}

pub(crate) trait ApiResponseError: Sized {
    fn invalid_token() -> Self;
    fn not_found() -> Self;
    fn unexpected_status(status: u16) -> Self;

    fn unexpected_status_with_message(status: u16, _message: String) -> Self {
        Self::unexpected_status(status)
    }

    fn from_response_with_not_found_first(response: reqwest::blocking::Response) -> Self {
        map_api_error_response(
            response,
            NotFoundBehavior::MapToNotFound,
            InvalidTokenCheck::AfterNotFound,
        )
    }

    fn from_response_with_token_first(response: reqwest::blocking::Response) -> Self {
        map_api_error_response(
            response,
            NotFoundBehavior::MapToNotFound,
            InvalidTokenCheck::BeforeNotFound,
        )
    }

    fn from_response_without_not_found(response: reqwest::blocking::Response) -> Self {
        map_api_error_response(
            response,
            NotFoundBehavior::KeepUnexpectedStatus,
            InvalidTokenCheck::BeforeNotFound,
        )
    }
}

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

fn map_api_error_response<E: ApiResponseError>(
    response: reqwest::blocking::Response,
    not_found_behavior: NotFoundBehavior,
    invalid_token_check: InvalidTokenCheck,
) -> E {
    let status = response.status().as_u16();

    if matches!(invalid_token_check, InvalidTokenCheck::AfterNotFound)
        && status == 404
        && matches!(not_found_behavior, NotFoundBehavior::MapToNotFound)
    {
        return E::not_found();
    }

    let error_message = parse_api_error_message(response);

    if response_indicates_invalid_token(status, error_message.as_deref()) {
        return E::invalid_token();
    }

    if status == 404 && matches!(not_found_behavior, NotFoundBehavior::MapToNotFound) {
        return E::not_found();
    }

    if let Some(message) = error_message {
        return E::unexpected_status_with_message(status, message);
    }

    E::unexpected_status(status)
}

pub(crate) fn response_indicates_invalid_token(status: u16, error_message: Option<&str>) -> bool {
    if status == 401 {
        return true;
    }

    if status != 400 {
        return false;
    }

    error_message.is_some_and(message_indicates_invalid_token)
}

fn message_indicates_invalid_token(message: &str) -> bool {
    let message = message.to_ascii_lowercase();

    message.contains("401")
        || message.contains("unauthorized")
        || message.contains("invalid token")
        || message.contains("invalid access token")
        || message.contains("access token")
        || message.contains("access_token")
}

pub(crate) fn resolve_base_url(value: Option<String>) -> String {
    value
        .unwrap_or_else(|| "https://gitee.com/api".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{resolve_base_url, response_indicates_invalid_token};

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

    #[test]
    fn maps_unauthorized_status_to_invalid_token() {
        assert!(response_indicates_invalid_token(401, None));
    }

    #[test]
    fn maps_explicit_access_token_message_to_invalid_token() {
        assert!(response_indicates_invalid_token(
            400,
            Some("invalid access token")
        ));
    }

    #[test]
    fn does_not_map_generic_bad_request_token_text_to_invalid_token() {
        assert!(!response_indicates_invalid_token(
            400,
            Some("title token placeholder cannot be blank")
        ));
    }
}
