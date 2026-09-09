//! SCIM error types ([RFC 7644, section 3.12]).
//!
//! [RFC 7644, section 3.12]: https://datatracker.ietf.org/doc/html/rfc7644#section-3.12

use serde::Serialize;

/// Errors produced by SCIM operations.
///
/// Each variant maps to the HTTP status codes and error payloads described
/// by RFC 7644 section 3.12.
#[derive(Debug, thiserror::Error)]
pub enum ScimError {
    /// The requested resource does not exist (HTTP 404).
    #[error("Not found")]
    NotFound,
    /// The request would create a duplicate resource (HTTP 409).
    #[error("Conflict: {0}")]
    Conflict(String),
    /// The request payload is malformed or invalid (HTTP 400).
    #[error("Bad request: {0}")]
    BadRequest(String),
    /// Authentication is required or failed (HTTP 401).
    #[error("Unauthorized")]
    Unauthorized,
    /// An unexpected server-side failure (HTTP 500).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ScimError {
    /// Returns the HTTP status code for this error.
    pub fn status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Conflict(_) => 409,
            Self::BadRequest(_) => 400,
            Self::Unauthorized => 401,
            Self::Internal(_) => 500,
        }
    }

    /// Returns the SCIM `scimType` keyword for this error, if any.
    pub fn scim_type(&self) -> Option<String> {
        match self {
            Self::Conflict(_) => Some("invalidValue".to_string()),
            _ => None,
        }
    }

    /// Builds the SCIM error response payload for this error.
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            schemas: vec![crate::schema::ERROR_URN.to_string()],
            scim_type: self.scim_type(),
            detail: self.to_string(),
            status: self.status(),
        }
    }
}

/// The SCIM error response message
/// (`urn:ietf:params:scim:api:messages:2.0:Error`).
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    /// Schema URIs; always contains the Error URN.
    pub schemas: Vec<String>,
    /// Optional SCIM detail keyword (e.g. `invalidValue`).
    #[serde(rename = "scimType", skip_serializing_if = "Option::is_none")]
    pub scim_type: Option<String>,
    /// Human-readable description of the error.
    pub detail: String,
    /// HTTP status code.
    pub status: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scim_error_not_found() {
        let err = ScimError::NotFound;
        assert_eq!(format!("{err}"), "Not found");
    }

    #[test]
    fn test_scim_error_conflict() {
        let err = ScimError::Conflict("exists".to_string());
        assert!(format!("{err}").contains("exists"));
    }

    #[test]
    fn test_scim_error_bad_request() {
        let err = ScimError::BadRequest("invalid".to_string());
        assert!(format!("{err}").contains("invalid"));
    }

    #[test]
    fn test_scim_error_unauthorized() {
        let err = ScimError::Unauthorized;
        assert_eq!(format!("{err}"), "Unauthorized");
    }

    #[test]
    fn test_scim_error_internal() {
        let err = ScimError::Internal("oops".to_string());
        assert!(format!("{err}").contains("oops"));
    }

    #[test]
    fn test_scim_error_debug() {
        let debug = format!("{:?}", ScimError::NotFound);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_scim_error_status_codes() {
        assert_eq!(ScimError::NotFound.status(), 404);
        assert_eq!(ScimError::Conflict("x".into()).status(), 409);
        assert_eq!(ScimError::BadRequest("x".into()).status(), 400);
        assert_eq!(ScimError::Unauthorized.status(), 401);
        assert_eq!(ScimError::Internal("x".into()).status(), 500);
    }

    #[test]
    fn test_scim_error_scim_type() {
        assert_eq!(
            ScimError::Conflict("x".into()).scim_type().as_deref(),
            Some("invalidValue")
        );
        assert_eq!(ScimError::NotFound.scim_type(), None);
    }

    #[test]
    fn test_error_response_serialization() {
        let body = ScimError::Conflict("exists".to_string()).to_error_response();
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(
            json["schemas"][0],
            "urn:ietf:params:scim:api:messages:2.0:Error"
        );
        assert_eq!(json["scimType"], "invalidValue");
        assert_eq!(json["status"], 409);
        assert!(json["detail"].as_str().unwrap().contains("exists"));
    }

    #[test]
    fn test_error_response_without_scim_type() {
        let body = ScimError::NotFound.to_error_response();
        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("scimType").is_none());
        assert_eq!(json["status"], 404);
    }
}
