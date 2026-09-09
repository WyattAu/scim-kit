//! SCIM core resource schemas ([RFC 7643]) and protocol messages
//! ([RFC 7644, section 3]).
//!
//! [RFC 7643]: https://datatracker.ietf.org/doc/html/rfc7643
//! [RFC 7644, section 3]: https://datatracker.ietf.org/doc/html/rfc7644#section-3

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core schema URN for User resources.
pub const USER_SCHEMA_URN: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
/// Core schema URN for Group resources.
pub const GROUP_SCHEMA_URN: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
/// Schema URN for `ListResponse` messages.
pub const LIST_RESPONSE_URN: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
/// Schema URN for `PatchOp` messages.
pub const PATCH_OP_URN: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
/// Schema URN for `Error` messages.
pub const ERROR_URN: &str = "urn:ietf:params:scim:api:messages:2.0:Error";
/// Schema URN for the service provider configuration document.
pub const SP_CONFIG_URN: &str = "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig";
/// Schema URN for schema definitions.
pub const SCHEMA_URN: &str = "urn:ietf:params:scim:schemas:core:2.0:Schema";
/// Schema URN for resource type definitions.
pub const RESOURCE_TYPE_URN: &str = "urn:ietf:params:scim:schemas:core:2.0:ResourceType";

/// Common behavior implemented by all SCIM resources.
pub trait Resource: Serialize {
    /// The resource type name (e.g. `"User"`).
    const RESOURCE_TYPE: &'static str;
    /// The core schema URN for this resource type.
    const SCHEMA_URN: &'static str;

    /// The unique identifier of the resource.
    fn id(&self) -> &str;
    /// The resource metadata.
    fn meta(&self) -> &ScimMeta;
}

/// Resource metadata (`meta` attribute).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimMeta {
    /// The resource type name.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Creation timestamp.
    pub created: DateTime<Utc>,
    /// Last modification timestamp.
    #[serde(rename = "lastModified")]
    pub last_modified: DateTime<Utc>,
    /// URI of the resource.
    pub location: String,
}

/// A person's name (`name` attribute of User).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimName {
    /// The given (first) name.
    #[serde(rename = "givenName", skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    /// The family (last) name.
    #[serde(rename = "familyName", skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    /// The full name as it should be displayed.
    #[serde(rename = "formatted", skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
}

/// An email address (`emails` entry of User).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimEmail {
    /// The email address.
    pub value: String,
    /// A label indicating the email's use (e.g. `work`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
    /// Whether this is the preferred email address.
    pub primary: bool,
}

/// A reference to another SCIM resource (group membership, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimUserRef {
    /// The identifier of the referenced resource.
    pub value: String,
    /// The URI of the referenced resource.
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_url: Option<String>,
    /// A human-readable name for the referenced resource.
    pub display: Option<String>,
}

/// A SCIM User resource ([RFC 7643, section 4.1], trimmed to common
/// provisioning attributes).
///
/// [RFC 7643, section 4.1]: https://datatracker.ietf.org/doc/html/rfc7643#section-4.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimUser {
    /// Schema URIs; typically contains [`USER_SCHEMA_URN`].
    pub schemas: Vec<String>,
    /// The unique identifier of the user.
    pub id: String,
    /// An identifier for the resource as defined by the provisioning client.
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// The unique username, intended as a login name.
    #[serde(rename = "userName")]
    pub user_name: String,
    /// The components of the user's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimName>,
    /// The name a user would prefer to be called.
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The user's email addresses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<ScimEmail>,
    /// Whether the account is active.
    pub active: bool,
    /// The groups this user belongs to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ScimUserRef>,
    /// Resource metadata.
    pub meta: ScimMeta,
}

impl Resource for ScimUser {
    const RESOURCE_TYPE: &'static str = "User";
    const SCHEMA_URN: &'static str = USER_SCHEMA_URN;

    fn id(&self) -> &str {
        &self.id
    }

    fn meta(&self) -> &ScimMeta {
        &self.meta
    }
}

/// A SCIM Group resource ([RFC 7643, section 4.2]).
///
/// [RFC 7643, section 4.2]: https://datatracker.ietf.org/doc/html/rfc7643#section-4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimGroup {
    /// Schema URIs; typically contains [`GROUP_SCHEMA_URN`].
    pub schemas: Vec<String>,
    /// The unique identifier of the group.
    pub id: String,
    /// A human-readable name for the group.
    #[serde(rename = "displayName")]
    pub display_name: String,
    /// The group's members.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<ScimUserRef>,
    /// Resource metadata.
    pub meta: ScimMeta,
}

impl Resource for ScimGroup {
    const RESOURCE_TYPE: &'static str = "Group";
    const SCHEMA_URN: &'static str = GROUP_SCHEMA_URN;

    fn id(&self) -> &str {
        &self.id
    }

    fn meta(&self) -> &ScimMeta {
        &self.meta
    }
}

/// A SCIM list response wrapping a page of resources
/// ([RFC 7644, section 3.4.2]).
///
/// `T` is the resource type carried in `resources`.
///
/// [RFC 7644, section 3.4.2]: https://datatracker.ietf.org/doc/html/rfc7644#section-3.4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimListResponse<T> {
    /// Schema URIs; typically contains [`LIST_RESPONSE_URN`].
    pub schemas: Vec<String>,
    /// The total number of results across all pages.
    #[serde(rename = "totalResults")]
    pub total_results: u32,
    /// The 1-based index of the first result in this page.
    #[serde(rename = "startIndex")]
    pub start_index: u32,
    /// The number of results returned in this page.
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: u32,
    /// The resources in this page.
    pub resources: Vec<T>,
}

impl<T> ScimListResponse<T> {
    /// Builds a list response with the standard `ListResponse` schema URN.
    pub fn new(
        resources: Vec<T>,
        total_results: u32,
        start_index: u32,
        items_per_page: u32,
    ) -> Self {
        Self {
            schemas: vec![LIST_RESPONSE_URN.to_string()],
            total_results,
            start_index,
            items_per_page,
            resources,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta() -> ScimMeta {
        ScimMeta {
            resource_type: "User".into(),
            created: Utc::now(),
            last_modified: Utc::now(),
            location: "/scim/v2/Users/test".into(),
        }
    }

    #[test]
    fn test_scim_user_serialize() {
        let user = ScimUser {
            schemas: vec![USER_SCHEMA_URN.into()],
            id: "u1".into(),
            external_id: None,
            user_name: "testuser".into(),
            name: None,
            display_name: Some("Test User".into()),
            emails: vec![],
            active: true,
            groups: vec![],
            meta: make_meta(),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("userName"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_scim_user_deserialize() {
        let json = r#"{
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "u1",
            "userName": "testuser",
            "active": true,
            "emails": [],
            "groups": [],
            "meta": {
                "resourceType": "User",
                "created": "2024-01-01T00:00:00Z",
                "lastModified": "2024-01-01T00:00:00Z",
                "location": "/scim/v2/Users/u1"
            }
        }"#;
        let user: ScimUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_name, "testuser");
        assert!(user.active);
    }

    #[test]
    fn test_scim_group_serialize() {
        let group = ScimGroup {
            schemas: vec![GROUP_SCHEMA_URN.into()],
            id: "g1".into(),
            display_name: "Admins".into(),
            members: vec![],
            meta: ScimMeta {
                resource_type: "Group".into(),
                created: Utc::now(),
                last_modified: Utc::now(),
                location: "/scim/v2/Groups/g1".into(),
            },
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("displayName"));
        assert!(json.contains("Admins"));
    }

    #[test]
    fn test_scim_list_response() {
        let response = ScimListResponse {
            schemas: vec![LIST_RESPONSE_URN.into()],
            total_results: 2,
            start_index: 1,
            items_per_page: 10,
            resources: vec!["item1".to_string(), "item2".to_string()],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("totalResults"));
        assert!(json.contains("startIndex"));
        assert!(json.contains("itemsPerPage"));
    }

    #[test]
    fn test_scim_list_response_new() {
        let response = ScimListResponse::new(vec![1, 2, 3], 3, 1, 3);
        assert_eq!(response.schemas, vec![LIST_RESPONSE_URN]);
        assert_eq!(response.total_results, 3);
        assert_eq!(response.resources.len(), 3);
    }

    #[test]
    fn test_scim_name_serialize() {
        let name = ScimName {
            given_name: Some("John".into()),
            family_name: Some("Doe".into()),
            formatted: Some("John Doe".into()),
        };
        let json = serde_json::to_string(&name).unwrap();
        assert!(json.contains("givenName"));
        assert!(json.contains("familyName"));
    }

    #[test]
    fn test_scim_email_serialize() {
        let email = ScimEmail {
            value: "test@example.com".into(),
            email_type: Some("work".into()),
            primary: true,
        };
        let json = serde_json::to_string(&email).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("work"));
    }

    #[test]
    fn test_scim_user_ref_serialize() {
        let user_ref = ScimUserRef {
            value: "u1".into(),
            ref_url: Some("/scim/v2/Users/u1".into()),
            display: Some("Test".into()),
        };
        let json = serde_json::to_string(&user_ref).unwrap();
        assert!(json.contains("$ref"));
    }

    #[test]
    fn test_scim_meta_debug() {
        let meta = make_meta();
        let debug = format!("{meta:?}");
        assert!(debug.contains("ScimMeta"));
    }

    #[test]
    fn test_scim_user_debug() {
        let user = ScimUser {
            schemas: vec![],
            id: "u1".into(),
            external_id: None,
            user_name: "test".into(),
            name: None,
            display_name: None,
            emails: vec![],
            active: true,
            groups: vec![],
            meta: make_meta(),
        };
        let debug = format!("{user:?}");
        assert!(debug.contains("ScimUser"));
    }

    #[test]
    fn test_resource_trait() {
        let user = ScimUser {
            schemas: vec![USER_SCHEMA_URN.into()],
            id: "u1".into(),
            external_id: None,
            user_name: "test".into(),
            name: None,
            display_name: None,
            emails: vec![],
            active: true,
            groups: vec![],
            meta: make_meta(),
        };
        assert_eq!(<ScimUser as Resource>::RESOURCE_TYPE, "User");
        assert_eq!(<ScimUser as Resource>::SCHEMA_URN, USER_SCHEMA_URN);
        assert_eq!(Resource::id(&user), "u1");
        assert_eq!(Resource::meta(&user).resource_type, "User");

        assert_eq!(<ScimGroup as Resource>::RESOURCE_TYPE, "Group");
        assert_eq!(<ScimGroup as Resource>::SCHEMA_URN, GROUP_SCHEMA_URN);
    }

    #[test]
    fn test_schema_urn_constants() {
        assert_eq!(
            USER_SCHEMA_URN,
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
        assert_eq!(
            GROUP_SCHEMA_URN,
            "urn:ietf:params:scim:schemas:core:2.0:Group"
        );
        assert_eq!(
            LIST_RESPONSE_URN,
            "urn:ietf:params:scim:api:messages:2.0:ListResponse"
        );
        assert_eq!(
            PATCH_OP_URN,
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
        );
        assert_eq!(ERROR_URN, "urn:ietf:params:scim:api:messages:2.0:Error");
        assert_eq!(
            SP_CONFIG_URN,
            "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"
        );
        assert_eq!(SCHEMA_URN, "urn:ietf:params:scim:schemas:core:2.0:Schema");
        assert_eq!(
            RESOURCE_TYPE_URN,
            "urn:ietf:params:scim:schemas:core:2.0:ResourceType"
        );
    }
}
