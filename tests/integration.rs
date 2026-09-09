//! End-to-end protocol tests exercising only the public API.

use chrono::Utc;
use scim_kit::{
    Filter, GroupStore, Operation, Pagination, PatchOp, PatchOperation, ScimGroup, ScimMeta,
    ScimUser, ERROR_URN, GROUP_SCHEMA_URN, LIST_RESPONSE_URN,
};

const USER_JSON: &str = r#"{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "2819c223-7f76-453a-919d-413861904646",
    "userName": "bjensen@example.com",
    "name": { "givenName": "Barbara", "familyName": "Jensen" },
    "active": true,
    "emails": [{ "value": "bjensen@example.com", "type": "work", "primary": true }],
    "meta": {
        "resourceType": "User",
        "created": "2024-01-01T00:00:00Z",
        "lastModified": "2024-01-01T00:00:00Z",
        "location": "/scim/v2/Users/2819c223-7f76-453a-919d-413861904646"
    }
}"#;

#[test]
fn test_user_round_trip_and_filter() {
    let user: ScimUser = serde_json::from_str(USER_JSON).unwrap();
    assert_eq!(user.user_name, "bjensen@example.com");

    let round_tripped: ScimUser =
        serde_json::from_str(&serde_json::to_string(&user).unwrap()).unwrap();
    assert_eq!(round_tripped.id, user.id);

    assert!(Filter::parse(r#"userName sw "bjensen""#)
        .unwrap()
        .matches_serialized(&round_tripped));
    assert!(
        Filter::parse(r#"name.familyName eq "jensen" and active eq true"#)
            .unwrap()
            .matches_serialized(&round_tripped)
    );
    assert!(!Filter::parse(r#"userName co "admin""#)
        .unwrap()
        .matches_serialized(&round_tripped));
}

#[test]
fn test_patch_user_reactivation() {
    let mut doc =
        serde_json::to_value(serde_json::from_str::<ScimUser>(USER_JSON).unwrap()).unwrap();
    let patch = PatchOp::new(vec![
        PatchOperation {
            op: Operation::Replace,
            path: Some("active".to_string()),
            value: Some(serde_json::json!(false)),
        },
        PatchOperation {
            op: Operation::Add,
            path: Some("displayName".to_string()),
            value: Some(serde_json::json!("Barb")),
        },
    ]);
    patch.apply_to(&mut doc).unwrap();
    assert_eq!(doc["active"], serde_json::json!(false));
    assert_eq!(doc["displayName"], serde_json::json!("Barb"));
}

#[test]
fn test_patch_group_member_removal() {
    let mut doc = serde_json::json!({
        "schemas": [GROUP_SCHEMA_URN],
        "displayName": "Developers",
        "members": [
            { "value": "u1", "display": "Babs" },
            { "value": "u2", "display": "Jay" }
        ]
    });
    let patch: PatchOp = serde_json::from_str(
        r#"{
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
            "Operations": [{
                "op": "remove",
                "path": "members[value eq \"u1\"]"
            }]
        }"#,
    )
    .unwrap();
    patch.apply_to(&mut doc).unwrap();
    let members = doc["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["value"], "u2");
}

#[test]
fn test_pagination_to_list_response() {
    let users: Vec<ScimUser> = (0..5)
        .map(|i| {
            let mut user: ScimUser = serde_json::from_str(USER_JSON).unwrap();
            user.id = format!("u{i}");
            user.user_name = format!("user{i}");
            user
        })
        .collect();
    let response = Pagination::new(1, 2).apply(&users).into_list_response();
    assert_eq!(response.total_results, 5);
    assert_eq!(response.resources.len(), 2);
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["schemas"][0], LIST_RESPONSE_URN);
    assert_eq!(json["totalResults"], 5);
    assert_eq!(json["itemsPerPage"], 2);
}

#[test]
fn test_error_response_shape() {
    use scim_kit::ScimError;
    let body = ScimError::BadRequest("invalid filter".to_string()).to_error_response();
    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["schemas"][0], ERROR_URN);
    assert_eq!(json["status"], 400);
    assert!(json["detail"].as_str().unwrap().contains("invalid filter"));
}

#[test]
fn test_group_store_with_filters_and_pagination() {
    let store = GroupStore::new();
    for name in ["Admins", "Devs", "QAs"] {
        store
            .create(ScimGroup {
                schemas: vec![GROUP_SCHEMA_URN.to_string()],
                id: name.to_lowercase(),
                display_name: name.to_string(),
                members: vec![],
                meta: ScimMeta {
                    resource_type: "Group".to_string(),
                    created: Utc::now(),
                    last_modified: Utc::now(),
                    location: format!("/scim/v2/Groups/{}", name.to_lowercase()),
                },
            })
            .unwrap();
    }
    assert_eq!(store.count(), 3);

    let page = Pagination::new(1, 2).apply(&store.list(0, 100));
    assert_eq!(page.resources.len(), 2);
    assert_eq!(page.total_results, 3);

    let all = store.list(0, 100);
    let devs: Vec<_> = all
        .iter()
        .filter(|g| {
            Filter::parse(r#"displayName eq "devs""#)
                .unwrap()
                .matches_serialized(g)
        })
        .collect();
    assert_eq!(devs.len(), 1);
    assert_eq!(devs[0].id, "devs");
}
