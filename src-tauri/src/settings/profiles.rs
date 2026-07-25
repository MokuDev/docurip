use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CrawlProfile {
    ApiDocs,
    Wiki,
    Blog,
    Documentation,
    Custom,
}

impl CrawlProfile {
    pub fn default_max_depth(&self) -> u32 {
        match self {
            CrawlProfile::ApiDocs => 3,
            CrawlProfile::Wiki => 4,
            CrawlProfile::Blog => 2,
            CrawlProfile::Documentation => 3,
            CrawlProfile::Custom => 2,
        }
    }

    pub fn default_page_limit(&self) -> u32 {
        match self {
            CrawlProfile::ApiDocs => 500,
            CrawlProfile::Wiki => 2000,
            CrawlProfile::Blog => 100,
            CrawlProfile::Documentation => 1000,
            CrawlProfile::Custom => 1000,
        }
    }

    pub fn default_content_selectors(&self) -> Vec<String> {
        match self {
            CrawlProfile::ApiDocs => vec![
                String::from("main"),
                String::from("[role='main']"),
                String::from(".api-content"),
                String::from("#api-reference"),
            ],
            CrawlProfile::Wiki => vec![
                String::from("main"),
                String::from("article"),
                String::from("[role='main']"),
                String::from("#content"),
                String::from(".wiki-content"),
            ],
            CrawlProfile::Blog => vec![
                String::from("article"),
                String::from(".post-content"),
                String::from(".entry-content"),
                String::from("main"),
            ],
            CrawlProfile::Documentation => vec![
                String::from("main"),
                String::from("article"),
                String::from("[role='main']"),
                String::from("#content"),
                String::from(".content"),
            ],
            CrawlProfile::Custom => vec![],
        }
    }

    pub fn default_exclude_patterns(&self) -> Vec<String> {
        match self {
            CrawlProfile::ApiDocs => vec![
                String::from(r".*/login.*"),
                String::from(r".*/signup.*"),
                String::from(r".*/changelog.*"),
            ],
            CrawlProfile::Wiki => vec![
                String::from(r".*/talk/.*"),
                String::from(r".*/user:.*"),
                String::from(r".*/special:.*"),
            ],
            CrawlProfile::Blog => vec![
                String::from(r".*/comments.*"),
                String::from(r".*/tag/.*"),
                String::from(r".*/category/.*"),
                String::from(r".*/author/.*"),
            ],
            CrawlProfile::Documentation => vec![
                String::from(r".*/login.*"),
                String::from(r".*/signup.*"),
            ],
            CrawlProfile::Custom => vec![],
        }
    }

    pub fn default_respect_robots_txt(&self) -> bool {
        match self {
            CrawlProfile::ApiDocs => true,
            CrawlProfile::Wiki => true,
            CrawlProfile::Blog => false,
            CrawlProfile::Documentation => true,
            CrawlProfile::Custom => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_docs_defaults() {
        let profile = CrawlProfile::ApiDocs;
        assert_eq!(profile.default_max_depth(), 3);
        assert_eq!(profile.default_page_limit(), 500);
        assert!(profile.default_content_selectors().contains(&String::from(".api-content")));
        assert!(profile.default_respect_robots_txt());
    }

    #[test]
    fn test_wiki_defaults() {
        let profile = CrawlProfile::Wiki;
        assert_eq!(profile.default_max_depth(), 4);
        assert_eq!(profile.default_page_limit(), 2000);
        assert!(profile.default_content_selectors().contains(&String::from(".wiki-content")));
        assert!(profile.default_respect_robots_txt());
    }

    #[test]
    fn test_blog_defaults() {
        let profile = CrawlProfile::Blog;
        assert_eq!(profile.default_max_depth(), 2);
        assert_eq!(profile.default_page_limit(), 100);
        assert!(profile.default_content_selectors().contains(&String::from("article")));
        assert!(!profile.default_respect_robots_txt());
    }

    #[test]
    fn test_documentation_defaults() {
        let profile = CrawlProfile::Documentation;
        assert_eq!(profile.default_max_depth(), 3);
        assert_eq!(profile.default_page_limit(), 1000);
        assert!(profile.default_content_selectors().contains(&String::from("main")));
        assert!(profile.default_respect_robots_txt());
    }

    #[test]
    fn test_custom_defaults() {
        let profile = CrawlProfile::Custom;
        assert_eq!(profile.default_max_depth(), 2);
        assert_eq!(profile.default_page_limit(), 1000);
        assert!(profile.default_content_selectors().is_empty());
        assert!(profile.default_exclude_patterns().is_empty());
        assert!(profile.default_respect_robots_txt());
    }

    #[test]
    fn test_profile_serialization() {
        let profile = CrawlProfile::ApiDocs;
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(json, r#""apiDocs""#);

        let deserialized: CrawlProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CrawlProfile::ApiDocs);
    }
}
