use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone)]
struct RobotsRule {
    allow: Vec<String>,
    disallow: Vec<String>,
    crawl_delay: Option<f64>,
}

impl RobotsRule {
    fn new() -> Self {
        Self {
            allow: Vec::new(),
            disallow: Vec::new(),
            crawl_delay: None,
        }
    }

    fn is_allowed(&self, path: &str) -> bool {
        let mut blocked = false;
        for d in &self.disallow {
            if d == "/" {
                blocked = true;
            } else if path.starts_with(d.as_str()) {
                blocked = true;
            }
        }
        if blocked {
            for a in &self.allow {
                if path.starts_with(a.as_str()) {
                    return true;
                }
            }
            return false;
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct RobotsTxt {
    rules: RobotsRule,
    fetched: bool,
}

impl Default for RobotsTxt {
    fn default() -> Self {
        Self {
            rules: RobotsRule::new(),
            fetched: false,
        }
    }
}

impl RobotsTxt {
    pub fn parse(content: &str, user_agent: &str) -> Self {
        let mut groups: HashMap<String, RobotsRule> = HashMap::new();
        let mut current_agents: Vec<String> = Vec::new();
        let mut wildcard_rule = RobotsRule::new();
        let mut has_wildcard = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let lower = line.to_lowercase();
            if let Some(val) = lower.strip_prefix("user-agent:") {
                let val = val.trim().to_string();
                current_agents.clear();
                current_agents.push(val);
            } else if let Some(val) = lower.strip_prefix("disallow:") {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    for agent in &current_agents {
                        if agent == "*" {
                            has_wildcard = true;
                            wildcard_rule.disallow.push(val.clone());
                        } else {
                            groups.entry(agent.clone()).or_insert_with(RobotsRule::new).disallow.push(val.clone());
                        }
                    }
                }
            } else if let Some(val) = lower.strip_prefix("allow:") {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    for agent in &current_agents {
                        if agent == "*" {
                            has_wildcard = true;
                            wildcard_rule.allow.push(val.clone());
                        } else {
                            groups.entry(agent.clone()).or_insert_with(RobotsRule::new).allow.push(val.clone());
                        }
                    }
                }
            } else if let Some(val) = lower.strip_prefix("crawl-delay:") {
                if let Ok(delay) = val.trim().parse::<f64>() {
                    for agent in &current_agents {
                        if agent == "*" {
                            has_wildcard = true;
                            wildcard_rule.crawl_delay = Some(delay);
                        } else {
                            groups.entry(agent.clone()).or_insert_with(RobotsRule::new).crawl_delay = Some(delay);
                        }
                    }
                }
            }
        }

        let rules = if let Some(specific) = groups.get(&user_agent.to_lowercase()) {
            specific.clone()
        } else if has_wildcard {
            wildcard_rule
        } else {
            RobotsRule::new()
        };

        Self {
            rules,
            fetched: true,
        }
    }

    pub fn is_allowed(&self, url: &str) -> bool {
        if !self.fetched {
            return true;
        }
        let path = match Url::parse(url) {
            Ok(u) => u.path().to_string(),
            Err(_) => "/".to_string(),
        };
        self.rules.is_allowed(&path)
    }

    pub fn crawl_delay_secs(&self) -> Option<u64> {
        self.rules.crawl_delay.map(|d| d.ceil() as u64)
    }

    pub fn was_fetched(&self) -> bool {
        self.fetched
    }
}

pub async fn fetch_robots_txt(base_url: &Url, user_agent: &str) -> RobotsTxt {
    let mut robots_url = base_url.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    match client
        .get(robots_url.as_str())
        .header("User-Agent", user_agent)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(body) => RobotsTxt::parse(&body, user_agent),
                Err(_) => RobotsTxt::default(),
            }
        }
        _ => RobotsTxt::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_robots_txt() {
        let content = "User-agent: *\nDisallow: /admin/\nDisallow: /private/\nAllow: /admin/public/\nCrawl-delay: 5\n";
        let robots = RobotsTxt::parse(content, "*");
        assert!(!robots.is_allowed("https://example.com/admin/settings"));
        assert!(robots.is_allowed("https://example.com/admin/public/info"));
        assert!(robots.is_allowed("https://example.com/docs/intro"));
        assert!(!robots.is_allowed("https://example.com/private/data"));
        assert_eq!(robots.crawl_delay_secs(), Some(5));
    }

    #[test]
    fn parse_specific_user_agent() {
        let content = "User-agent: Googlebot\nDisallow: /secret/\n\nUser-agent: *\nDisallow: /tmp/\n";
        let robots = RobotsTxt::parse(content, "Googlebot");
        assert!(!robots.is_allowed("https://example.com/secret/page"));
        assert!(robots.is_allowed("https://example.com/tmp/page"));
    }

    #[test]
    fn empty_robots_allows_all() {
        let robots = RobotsTxt::default();
        assert!(robots.is_allowed("https://example.com/anything"));
    }

    #[test]
    fn disallow_root_blocks_everything() {
        let content = "User-agent: *\nDisallow: /\n";
        let robots = RobotsTxt::parse(content, "*");
        assert!(!robots.is_allowed("https://example.com/anything"));
    }

    #[test]
    fn allow_overrides_disallow() {
        let content = "User-agent: *\nDisallow: /admin/\nAllow: /admin/public/\n";
        let robots = RobotsTxt::parse(content, "*");
        assert!(!robots.is_allowed("https://example.com/admin/secret"));
        assert!(robots.is_allowed("https://example.com/admin/public/page"));
    }

    #[test]
    fn was_fetched_flag() {
        let robots = RobotsTxt::default();
        assert!(!robots.was_fetched());
        let robots = RobotsTxt::parse("User-agent: *\nDisallow: /\n", "*");
        assert!(robots.was_fetched());
    }

    #[test]
    fn parse_ignores_comments_and_empty_lines() {
        let content = "# This is a comment\n\nUser-agent: *\n# Another comment\nDisallow: /test/\n";
        let robots = RobotsTxt::parse(content, "*");
        assert!(!robots.is_allowed("https://example.com/test/page"));
        assert!(robots.is_allowed("https://example.com/other/page"));
    }

    #[test]
    fn disallow_empty_value_not_blocked() {
        let content = "User-agent: *\nDisallow:\n";
        let robots = RobotsTxt::parse(content, "*");
        assert!(robots.is_allowed("https://example.com/anything"));
    }
}
