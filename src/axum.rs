//! Optional [`axum`] integration: a ready-made SCIM v2 HTTP router.
//!
//! Enable the `axum` feature to use this module. The handlers are a
//! reference implementation backed by the in-memory [`GroupStore`]; mount
//! [`routes`] under your own authentication middleware.

use crate::error::ScimError;
use crate::schema::{
    ScimGroup, ScimListResponse, ScimUser, GROUP_SCHEMA_URN, RESOURCE_TYPE_URN, SCHEMA_URN,
    SP_CONFIG_URN, USER_SCHEMA_URN,
};
use crate::GroupStore;
use ::axum::extract::{Path, State};
use ::axum::http::StatusCode;
use ::axum::response::{IntoResponse, Response};
use ::axum::routing::get;
use ::axum::{Json, Router};
use serde_json::Value;
use std::sync::Arc;

/// Shared state for the SCIM router.
#[derive(Clone, Default)]
pub struct ScimState {
    /// The in-memory group store backing the group endpoints.
    pub group_store: Arc<GroupStore>,
}

impl ScimState {
    /// Creates state with an empty group store.
    pub fn new() -> Self {
        Self {
            group_store: Arc::new(GroupStore::new()),
        }
    }
}

/// Builds the SCIM v2 router (`/scim/v2/...`).
pub fn routes(state: ScimState) -> Router {
    Router::new()
        .route("/scim/v2/Users", get(list_users).post(create_user))
        .route(
            "/scim/v2/Users/{id}",
            get(get_user).put(replace_user).delete(delete_user),
        )
        .route("/scim/v2/Groups", get(list_groups).post(create_group))
        .route(
            "/scim/v2/Groups/{id}",
            get(get_group).put(replace_group).delete(delete_group),
        )
        .route("/scim/v2/ServiceProviderConfig", get(sp_config))
        .route("/scim/v2/Schemas", get(schemas))
        .route("/scim/v2/ResourceTypes", get(resource_types))
        .with_state(state)
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.to_error_response())).into_response()
    }
}

/// Lists users. Returns an empty list; supply your own user store.
pub async fn list_users(
    State(_state): State<ScimState>,
) -> Result<Json<ScimListResponse<ScimUser>>, ScimError> {
    Ok(Json(ScimListResponse::new(vec![], 0, 1, 0)))
}

/// Creates a user, echoing the payload with HTTP 201.
pub async fn create_user(
    State(_state): State<ScimState>,
    Json(user): Json<ScimUser>,
) -> Result<(StatusCode, Json<ScimUser>), ScimError> {
    Ok((StatusCode::CREATED, Json(user)))
}

/// Fetches a user by id. Always returns 404; supply your own user store.
pub async fn get_user(
    Path(_id): Path<String>,
    State(_state): State<ScimState>,
) -> Result<Json<ScimUser>, ScimError> {
    Err(ScimError::NotFound)
}

/// Replaces a user, echoing the payload.
pub async fn replace_user(
    Path(_id): Path<String>,
    State(_state): State<ScimState>,
    Json(user): Json<ScimUser>,
) -> Result<Json<ScimUser>, ScimError> {
    Ok(Json(user))
}

/// Deletes a user, returning 204.
pub async fn delete_user(
    Path(_id): Path<String>,
    State(_state): State<ScimState>,
) -> Result<StatusCode, ScimError> {
    Ok(StatusCode::NO_CONTENT)
}

/// Lists groups from the in-memory store.
pub async fn list_groups(
    State(state): State<ScimState>,
) -> Result<Json<ScimListResponse<ScimGroup>>, ScimError> {
    let groups = state.group_store.list(0, 100);
    let total = state.group_store.count();
    Ok(Json(ScimListResponse::new(groups, total, 1, total)))
}

/// Creates a group in the in-memory store, returning HTTP 201.
pub async fn create_group(
    State(state): State<ScimState>,
    Json(group): Json<ScimGroup>,
) -> Result<(StatusCode, Json<ScimGroup>), ScimError> {
    let created = state.group_store.create(group)?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Fetches a group by id.
pub async fn get_group(
    Path(id): Path<String>,
    State(state): State<ScimState>,
) -> Result<Json<ScimGroup>, ScimError> {
    let group = state.group_store.get(&id)?;
    Ok(Json(group))
}

/// Replaces a group in the in-memory store.
pub async fn replace_group(
    Path(id): Path<String>,
    State(state): State<ScimState>,
    Json(group): Json<ScimGroup>,
) -> Result<Json<ScimGroup>, ScimError> {
    let updated = state.group_store.update(&id, group)?;
    Ok(Json(updated))
}

/// Deletes a group from the in-memory store, returning 204.
pub async fn delete_group(
    Path(id): Path<String>,
    State(state): State<ScimState>,
) -> Result<StatusCode, ScimError> {
    state.group_store.delete(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Returns the service provider configuration document.
pub async fn sp_config() -> Json<Value> {
    Json(serde_json::json!({
        "schemas": [SP_CONFIG_URN],
        "patch": { "supported": false },
        "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
        "filter": { "supported": false, "maxResults": 0 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
    }))
}

/// Returns the supported schema definitions.
pub async fn schemas() -> Json<Value> {
    Json(serde_json::json!({
        "schemas": [SCHEMA_URN],
        "totalResults": 2,
        "Resources": [
            { "id": USER_SCHEMA_URN, "name": "User" },
            { "id": GROUP_SCHEMA_URN, "name": "Group" },
        ],
    }))
}

/// Returns the supported resource type definitions.
pub async fn resource_types() -> Json<Value> {
    Json(serde_json::json!({
        "schemas": [RESOURCE_TYPE_URN],
        "totalResults": 2,
        "Resources": [
            { "id": "User", "name": "User", "endpoint": "/scim/v2/Users", "schema": USER_SCHEMA_URN },
            { "id": "Group", "name": "Group", "endpoint": "/scim/v2/Groups", "schema": GROUP_SCHEMA_URN },
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ScimMeta;
    use chrono::Utc;

    fn make_group(id: &str, name: &str) -> ScimGroup {
        ScimGroup {
            schemas: vec![GROUP_SCHEMA_URN.to_string()],
            id: id.to_string(),
            display_name: name.to_string(),
            members: vec![],
            meta: ScimMeta {
                resource_type: "Group".to_string(),
                created: Utc::now(),
                last_modified: Utc::now(),
                location: format!("/scim/v2/Groups/{id}"),
            },
        }
    }

    #[test]
    fn test_error_into_response_status() {
        let response = ScimError::Conflict("dup".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = ScimError::NotFound.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_and_get_group_via_state() {
        let state = ScimState::new();
        let created = state.group_store.create(make_group("g1", "Admins"));
        assert!(created.is_ok());
        let fetched = state.group_store.get("g1");
        assert!(fetched.is_ok());
    }

    #[test]
    fn test_routes_builds() {
        let _router = routes(ScimState::new());
    }
}
