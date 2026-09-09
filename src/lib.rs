#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # scim-kit
//!
//! A self-contained SCIM 2.0 provisioning toolkit implementing the protocol
//! pieces of [RFC 7644]: resource types ([`ScimUser`], [`ScimGroup`]),
//! list responses, error responses, filtering, pagination, and patching.
//!
//! The core crate has no HTTP framework dependencies. An optional [`axum`]
//! feature provides a ready-made SCIM v2 router.
//!
//! [RFC 7644]: https://datatracker.ietf.org/doc/html/rfc7644
//!
//! # Example
//!
//! ```
//! use scim_kit::{Filter, ScimUser};
//!
//! let user: ScimUser = serde_json::from_str(
//!     r#"{
//!         "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//!         "id": "u1",
//!         "userName": "bjensen",
//!         "active": true,
//!         "meta": {
//!             "resourceType": "User",
//!             "created": "2024-01-01T00:00:00Z",
//!             "lastModified": "2024-01-01T00:00:00Z",
//!             "location": "/scim/v2/Users/u1"
//!         }
//!     }"#,
//! )
//! .unwrap();
//!
//! let filter = Filter::parse(r#"userName eq "BJENSEN""#).unwrap();
//! assert!(filter.matches_serialized(&user));
//! ```

pub mod error;
pub mod filter;
pub mod group;
pub mod pagination;
pub mod patch;
pub mod schema;
pub mod user;

#[cfg(feature = "axum")]
pub mod axum;

pub use error::{ErrorResponse, ScimError};
pub use filter::{CompareOp, Filter};
pub use group::GroupStore;
pub use pagination::{Page, Pagination};
pub use patch::{Operation, PatchOp, PatchOperation};
pub use schema::{
    Resource, ScimEmail, ScimGroup, ScimListResponse, ScimMeta, ScimName, ScimUser, ScimUserRef,
    ERROR_URN, GROUP_SCHEMA_URN, LIST_RESPONSE_URN, PATCH_OP_URN, RESOURCE_TYPE_URN, SCHEMA_URN,
    SP_CONFIG_URN, USER_SCHEMA_URN,
};
pub use user::to_scim_user;
