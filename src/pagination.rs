//! SCIM pagination ([RFC 7644, section 3.4.2.3]).
//!
//! [RFC 7644, section 3.4.2.3]: https://datatracker.ietf.org/doc/html/rfc7644#section-3.4.2.3

use crate::schema::ScimListResponse;

/// The page size used when a request does not specify `count`.
pub const DEFAULT_PAGE_SIZE: u32 = 100;
/// The maximum accepted page size; larger `count` values are clamped.
pub const MAX_PAGE_SIZE: u32 = 1000;

/// Pagination parameters (`startIndex` / `count`) of a list request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    /// The 1-based index of the first result to return.
    pub start_index: u32,
    /// The maximum number of results to return per page.
    pub count: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            start_index: 1,
            count: DEFAULT_PAGE_SIZE,
        }
    }
}

impl Pagination {
    /// Creates pagination parameters, clamping `start_index` to at least 1
    /// and `count` to [`MAX_PAGE_SIZE`].
    pub fn new(start_index: u32, count: u32) -> Self {
        Self {
            start_index: start_index.max(1),
            count: count.min(MAX_PAGE_SIZE),
        }
    }

    /// Parses pagination parameters from a URL query string (the part after
    /// `?`), ignoring unknown keys and unparseable values.
    pub fn from_query(query: &str) -> Self {
        let mut page = Self::default();
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key.to_ascii_lowercase().as_str() {
                "startindex" => {
                    if let Ok(n) = value.parse::<u32>() {
                        page.start_index = n.max(1);
                    }
                }
                "count" => {
                    if let Ok(n) = value.parse::<u32>() {
                        page.count = n.min(MAX_PAGE_SIZE);
                    }
                }
                _ => {}
            }
        }
        page
    }

    /// Slices `items` into a single [`Page`] according to these parameters.
    pub fn apply<T: Clone>(&self, items: &[T]) -> Page<T> {
        let offset = (self.start_index as usize - 1).min(items.len());
        let resources: Vec<T> = items[offset..]
            .iter()
            .take(self.count as usize)
            .cloned()
            .collect();
        Page {
            items_per_page: resources.len() as u32,
            resources,
            total_results: items.len() as u32,
            start_index: self.start_index,
        }
    }
}

/// A single page of results produced by [`Pagination::apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The resources in this page.
    pub resources: Vec<T>,
    /// The total number of results across all pages.
    pub total_results: u32,
    /// The 1-based index of the first result in this page.
    pub start_index: u32,
    /// The number of results returned in this page.
    pub items_per_page: u32,
}

impl<T> Page<T> {
    /// Converts this page into a SCIM [`ScimListResponse`].
    pub fn into_list_response(self) -> ScimListResponse<T> {
        ScimListResponse::new(
            self.resources,
            self.total_results,
            self.start_index,
            self.items_per_page,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<u32> {
        vec![1, 2, 3, 4, 5]
    }

    #[test]
    fn test_default() {
        let page = Pagination::default();
        assert_eq!(page.start_index, 1);
        assert_eq!(page.count, DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn test_new_clamps() {
        let page = Pagination::new(0, MAX_PAGE_SIZE + 500);
        assert_eq!(page.start_index, 1);
        assert_eq!(page.count, MAX_PAGE_SIZE);
    }

    #[test]
    fn test_from_query_empty() {
        let page = Pagination::from_query("");
        assert_eq!(page, Pagination::default());
    }

    #[test]
    fn test_from_query_values() {
        let page = Pagination::from_query("count=2&startIndex=3&filter=x");
        assert_eq!(page.start_index, 3);
        assert_eq!(page.count, 2);
    }

    #[test]
    fn test_from_query_ignores_invalid() {
        let page = Pagination::from_query("startIndex=abc&count=-5");
        assert_eq!(page, Pagination::default());
    }

    #[test]
    fn test_apply_first_page() {
        let page = Pagination::new(1, 2).apply(&items());
        assert_eq!(page.resources, vec![1, 2]);
        assert_eq!(page.total_results, 5);
        assert_eq!(page.items_per_page, 2);
    }

    #[test]
    fn test_apply_middle_page() {
        let page = Pagination::new(3, 2).apply(&items());
        assert_eq!(page.resources, vec![3, 4]);
    }

    #[test]
    fn test_apply_beyond_end() {
        let page = Pagination::new(99, 10).apply(&items());
        assert!(page.resources.is_empty());
        assert_eq!(page.total_results, 5);
    }

    #[test]
    fn test_into_list_response() {
        let response = Pagination::new(2, 2).apply(&items()).into_list_response();
        assert_eq!(response.total_results, 5);
        assert_eq!(response.start_index, 2);
        assert_eq!(response.items_per_page, 2);
        assert_eq!(response.resources, vec![2, 3]);
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["totalResults"], 5);
        assert_eq!(json["startIndex"], 2);
        assert_eq!(json["itemsPerPage"], 2);
    }
}
