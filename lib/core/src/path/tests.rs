use alloc::{string::String, vec::Vec};

use super::*;
use crate::strings::Name;

#[test]
fn new_produces_target_without_namespace_or_fields() {
    let path = Path::new("users");

    assert_eq!(path.target_str(), "users");
    assert!(!path.has_namespace());
    assert!(!path.has_fields());
}

#[test]
fn namespace_sets_and_replaces_namespace() {
    let path = Path::new("users").namespace("draft").namespace("public");

    assert_eq!(path.namespace_str(), Some("public"));
}

#[test]
fn get_appends_field_segments() {
    let path = Path::new("users").get("address").get("city");

    assert_eq!(path.field_len(), 2);
    assert_eq!(path.field_strs().collect::<Vec<_>>(), ["address", "city"]);
}

#[test]
fn database_style_path() {
    let path = Path::new("users")
        .namespace("public")
        .get("address")
        .get("city");

    assert_eq!(path.namespace_str(), Some("public"));
    assert_eq!(path.target_str(), "users");
    assert_eq!(path.field_strs().collect::<Vec<_>>(), ["address", "city"]);
}

#[test]
fn filesystem_style_path_uses_flattened_target() {
    let path = Path::new("documents/reports").namespace("C:").get("title");

    assert_eq!(path.namespace_str(), Some("C:"));
    assert_eq!(path.target_str(), "documents/reports");
    assert_eq!(path.field_single(), Some("title"));
}

#[test]
fn rest_style_path_uses_flattened_target() {
    let path = Path::new("users/profile")
        .namespace("api.acme.com")
        .get("address")
        .get("street");

    assert_eq!(path.namespace_str(), Some("api.acme.com"));
    assert_eq!(path.target_str(), "users/profile");
    assert_eq!(path.field_strs().collect::<Vec<_>>(), ["address", "street"]);
}

#[test]
fn field_single_returns_none_for_empty_or_multi_segment_field() {
    assert_eq!(Path::new("users").field_single(), None);
    assert_eq!(Path::new("users").get("a").get("b").field_single(), None);
}

#[test]
fn generic_resolution_matches_name_convenience_methods() {
    let path = Path::new("users")
        .namespace("public")
        .get("address")
        .get("city");

    assert_eq!(path.resolve_namespace(&()), path.namespace_str());
    assert_eq!(path.resolve_target(&()), path.target_str());
    assert_eq!(
        path.resolve_fields(&()).collect::<Vec<_>>(),
        path.field_strs().collect::<Vec<_>>()
    );
}

#[test]
fn equality_is_by_segment_content() {
    let a = Path::new("users").namespace("public").get("email");
    let b = Path::new(Name::owned(String::from("users")))
        .namespace(Name::owned(String::from("public")))
        .get(Name::owned(String::from("email")));

    assert_eq!(a, b);
}

#[test]
fn different_namespaces_are_not_equal() {
    let a = Path::new("users").namespace("public");
    let b = Path::new("users").namespace("private");

    assert_ne!(a, b);
}

#[test]
fn different_targets_are_not_equal() {
    let a = Path::new("users");
    let b = Path::new("accounts");

    assert_ne!(a, b);
}

#[test]
fn different_fields_are_not_equal() {
    let a = Path::new("users").get("email");
    let b = Path::new("users").get("name");

    assert_ne!(a, b);
}

#[test]
fn name_static_and_owned_compare_by_content() {
    let static_name = Name::Static("users");
    let owned_name = Name::owned(String::from("users"));

    assert_eq!(static_name, owned_name);
    assert_eq!(static_name, "users");
    assert_eq!(owned_name.as_str(), "users");
}
