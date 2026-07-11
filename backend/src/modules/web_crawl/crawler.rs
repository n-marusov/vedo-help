use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::broadcast;
use tokio::time::sleep;
use uuid::Uuid;

use crate::modules::web_crawl::models::{CrawlConfig, CrawlProgress, CrawledPage};
use crate::shared::error::AppError;

/// Web crawler using BFS with same-domain enforcement, depth/pages limits,
/// robots.txt checking, and configurable rate limiting.
#[derive(Clone)]
pub struct WebCrawler {
    pub client: Client,
    pub robots_cache: Arc<RwLock<HashMap<String, RobotsTxtEntry>>>,
}

/// Cached robots.txt entry for a single domain.
#[derive(Clone)]
pub struct RobotsTxtEntry {
    pub disallowed_paths: Vec<String>,
    pub crawl_delay: Option<u64>,
    pub cached_at: std::time::Instant,
}

impl RobotsTxtEntry {
    const TTL: Duration = Duration::from_secs(3600);

    /// Check if the entry has expired.
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > Self::TTL
    }
}

impl WebCrawler {
    /// Create a new `WebCrawler` with default settings.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("VEDO-WebCrawler/1.0 (+https://vedo-hub.example.com)")
            .build()
            .expect("Failed to create HTTP client for WebCrawler");

        Self {
            client,
            robots_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Crawl a site starting from `entry_url` using BFS.
    ///
    /// Returns a list of crawled pages with extracted text content.
    /// Sends progress updates via `progress_tx` and respects cancellation via `cancel_rx`.
    #[allow(clippy::too_many_arguments)]
    pub async fn crawl(
        &self,
        entry_url: &str,
        config: &CrawlConfig,
        progress_tx: broadcast::Sender<CrawlProgress>,
        mut cancel_rx: broadcast::Receiver<()>,
        _collection_id: Uuid,
    ) -> Result<Vec<CrawledPage>, AppError> {
        let origin = Self::extract_origin(entry_url);
        let max_depth = config.max_depth;
        let max_pages = config.max_pages;
        let delay = Duration::from_millis(config.delay_ms);
        let path_prefix = &config.path_prefix;

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();
        let mut results: Vec<CrawledPage> = Vec::new();
        let base_depth = 0u32;

        // Normalize entry URL and add to queue
        let normalized_entry = normalize_url(entry_url);
        queue.push_back((normalized_entry.clone(), base_depth));

        let mut pages_discovered: i32 = 0;

        while let Some((url, depth)) = queue.pop_front() {
            // Check cancellation
            if cancel_rx.try_recv().is_ok() {
                tracing::info!(
                    component = "web_crawl/crawler",
                    entry_url = %entry_url,
                    pages_crawled = results.len(),
                    "crawl.cancelled"
                );
                break;
            }

            // Check max pages limit
            if !should_crawl_page(results.len() as u32, max_pages) {
                tracing::debug!(
                    component = "web_crawl/crawler",
                    pages_crawled = results.len(),
                    max_pages = max_pages,
                    "crawl.max_pages_reached"
                );
                break;
            }

            // Deduplicate
            if is_visited(&url, &visited) {
                continue;
            }
            visited.insert(url.clone());

            // Check same-domain
            if !is_same_domain(&url, &origin) {
                tracing::debug!(
                    component = "web_crawl/crawler",
                    url = %url,
                    "crawl.skipped_cross_domain"
                );
                continue;
            }

            // Check path prefix
            if !path_prefix.is_empty() && !matches_path_prefix(&url, path_prefix) {
                tracing::debug!(
                    component = "web_crawl/crawler",
                    url = %url,
                    "crawl.skipped_path_prefix"
                );
                continue;
            }

            // Check depth
            if !is_within_depth(depth, max_depth) {
                tracing::debug!(
                    component = "web_crawl/crawler",
                    url = %url,
                    depth = depth,
                    max_depth = max_depth,
                    "crawl.skipped_depth"
                );
                continue;
            }

            // Check robots.txt
            let domain = Self::extract_domain(&url);
            if let Some(domain) = domain {
                let allowed = self.check_robots_txt(&domain, &url).await.unwrap_or(true);
                if !allowed {
                    tracing::debug!(
                        component = "web_crawl/crawler",
                        url = %url,
                        "crawl.disallowed_by_robots"
                    );
                    continue;
                }
            }

            // Rate limit
            sleep(delay).await;

            // Update progress
            pages_discovered += 1;
            let progress = CrawlProgress {
                pages_found: pages_discovered,
                pages_indexed: 0,
                current_url: url.clone(),
                phase: "crawling".to_string(),
            };
            let _ = progress_tx.send(progress);

            // Fetch page
            let response = match self.client.get(&url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::warn!(
                        component = "web_crawl/crawler",
                        url = %url,
                        error = %e,
                        "crawl.fetch_failed"
                    );
                    continue;
                }
            };

            let http_status = response.status().as_u16() as i32;

            // Read body
            let body = match response.text().await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        component = "web_crawl/crawler",
                        url = %url,
                        error = %e,
                        "crawl.read_body_failed"
                    );
                    continue;
                }
            };

            // Extract content
            let extracted = ContentExtractor::extract(&body, &url);

            let crawled_page = CrawledPage {
                url: url.clone(),
                title: extracted.title,
                text: extracted.text,
                depth: depth,
                http_status: Some(http_status),
            };

            // Extract and enqueue links if within depth limit
            if depth < max_depth {
                let links = ContentExtractor::extract_links(&body, &url);
                for link in links {
                    let normalized = normalize_url(&link);
                    if !visited.contains(&normalized) {
                        queue.push_back((normalized, depth + 1));
                    }
                }
            }

            results.push(crawled_page);

            tracing::debug!(
                component = "web_crawl/crawler",
                url = %url,
                depth = depth,
                http_status = http_status,
                total_crawled = results.len(),
                "crawl.page_crawled"
            );
        }

        tracing::info!(
            component = "web_crawl/crawler",
            entry_url = %entry_url,
            pages_crawled = results.len(),
            "crawl.completed"
        );

        Ok(results)
    }

    /// Check if a URL is allowed by robots.txt for the given domain.
    /// Returns `true` if allowed or if robots.txt cannot be fetched.
    async fn check_robots_txt(&self, domain: &str, url: &str) -> Result<bool, AppError> {
        let path = extract_path(url);

        // Check cache
        {
            let cache = self
                .robots_cache
                .read()
                .map_err(|e| AppError::InternalError(format!("Robots cache lock error: {e}")))?;
            if let Some(entry) = cache.get(domain) {
                if !entry.is_expired() {
                    return Ok(Self::is_path_allowed(&entry.disallowed_paths, &path));
                }
            }
        }

        // Fetch robots.txt with 2-second timeout
        let robots_url = format!("https://{domain}/robots.txt");
        let fetch_result =
            tokio::time::timeout(Duration::from_secs(2), self.client.get(&robots_url).send()).await;

        match fetch_result {
            Ok(Ok(response)) if response.status().is_success() => {
                let body = response.text().await.unwrap_or_default();
                let entry = Self::parse_robots_txt(&body);
                let allowed = Self::is_path_allowed(&entry.disallowed_paths, &path);

                // Update cache
                if let Ok(mut cache) = self.robots_cache.write() {
                    cache.insert(domain.to_string(), entry);
                }

                Ok(allowed)
            }
            _ => {
                // Timeout or error: allow by default, cache a permissive entry briefly
                if let Ok(mut cache) = self.robots_cache.write() {
                    cache.insert(
                        domain.to_string(),
                        RobotsTxtEntry {
                            disallowed_paths: vec![],
                            crawl_delay: None,
                            cached_at: std::time::Instant::now(),
                        },
                    );
                }
                Ok(true)
            }
        }
    }

    /// Parse robots.txt content into a set of disallowed paths.
    fn parse_robots_txt(body: &str) -> RobotsTxtEntry {
        let mut disallowed_paths: Vec<String> = Vec::new();
        let mut crawl_delay: Option<u64> = None;
        let mut current_agent = String::new();

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.to_lowercase().strip_prefix("user-agent:") {
                current_agent = rest.trim().to_string();
                continue;
            }
            // Apply rules for * or our specific user agent
            if current_agent != "*" && current_agent != "vedo-webcrawler" && current_agent != "bot"
            {
                // Skip rules for other agents unless we're still on * rules
                // Actually, we should still collect * rules
                if current_agent != "*" {
                    // Skip non-matching agent entries
                    continue;
                }
            }
            if let Some(path) = line.strip_prefix("Disallow:") {
                let path = path.trim().to_string();
                if !path.is_empty() {
                    disallowed_paths.push(path);
                }
            }
            if let Some(delay_str) = line.strip_prefix("Crawl-delay:") {
                if let Ok(delay) = delay_str.trim().parse::<u64>() {
                    crawl_delay = Some(delay);
                }
            }
        }

        RobotsTxtEntry {
            disallowed_paths,
            crawl_delay,
            cached_at: std::time::Instant::now(),
        }
    }

    /// Check if a path is allowed given a list of disallowed paths.
    fn is_path_allowed(disallowed: &[String], path: &str) -> bool {
        for disallow in disallowed {
            if path.starts_with(disallow) {
                return false;
            }
        }
        true
    }

    /// Extract the origin (scheme + host) from a URL.
    fn extract_origin(url: &str) -> String {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 3 {
            format!("{}//{}", parts[0], parts[1])
        } else {
            url.to_string()
        }
    }

    /// Extract the domain (host) from a URL.
    fn extract_domain(url: &str) -> Option<String> {
        let after_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        after_scheme.split('/').next().map(|s| s.to_string())
    }
}

impl Default for WebCrawler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracted content from an HTML page.
#[derive(Debug)]
pub struct ExtractedContent {
    pub title: Option<String>,
    pub text: String,
}

/// Content extraction from HTML using `scraper`.
pub struct ContentExtractor;

impl ContentExtractor {
    /// Extract clean text content from an HTML page.
    ///
    /// Strips navigation, headers, footers, scripts, and styles.
    /// Preserves heading hierarchy as markdown and links as `[text](url)`.
    pub fn extract(html: &str, base_url: &str) -> ExtractedContent {
        let document = Html::parse_document(html);

        // Extract title
        let title = Self::extract_title(&document);

        // Remove unwanted elements by selecting common nav/header/footer patterns
        let body_text = Self::extract_body_text(&document, base_url);

        ExtractedContent {
            title,
            text: body_text,
        }
    }

    /// Extract the page title from <title> or <h1>.
    fn extract_title(document: &Html) -> Option<String> {
        // Try <title> first
        if let Ok(selector) = Selector::parse("title") {
            if let Some(el) = document.select(&selector).next() {
                let text = el.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        // Fallback to first <h1>
        if let Ok(selector) = Selector::parse("h1") {
            if let Some(el) = document.select(&selector).next() {
                let text = el.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        None
    }

    /// Extract clean body text, stripping unwanted elements.
    fn extract_body_text(document: &Html, base_url: &str) -> String {
        let mut text_parts: Vec<String> = Vec::new();
        let mut seen_text = std::collections::HashSet::new();

        // Try main content area first, then fall back to body
        let content_selectors = ["main", "article", "[role=main]", "body"];

        for sel_str in &content_selectors {
            if let Ok(selector) = Selector::parse(sel_str) {
                if let Some(root) = document.select(&selector).next() {
                    Self::collect_text_recursive(
                        &root,
                        document,
                        base_url,
                        0,
                        &mut text_parts,
                        &mut seen_text,
                    );
                    if !text_parts.is_empty() {
                        break;
                    }
                }
            }
        }

        text_parts.join("\n\n")
    }

    /// Recursively collect text from DOM nodes, with heading-to-markdown conversion.
    fn collect_text_recursive<'a>(
        node: &scraper::element_ref::ElementRef<'a>,
        document: &'a Html,
        base_url: &str,
        depth: usize,
        parts: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        // Skip unwanted elements by tag name
        let tag = node.value().name();
        let unwanted_tags = ["script", "style", "nav", "header", "footer", "aside"];
        if unwanted_tags.contains(&tag) {
            return;
        }

        // Skip elements with unwanted class names
        for class in node.value().classes() {
            let unwanted_classes = [
                "nav",
                "navbar",
                "header",
                "footer",
                "sidebar",
                "ad",
                "ads",
                "advertisement",
                "menu",
                "navigation",
                "cookie",
                "cookie-consent",
                "popup",
                "modal",
            ];
            if unwanted_classes.contains(&class) {
                return;
            }
        }

        let tag = node.value().name();

        // Handle headings → markdown
        let heading_level = match tag {
            "h1" => Some(1),
            "h2" => Some(2),
            "h3" => Some(3),
            "h4" => Some(4),
            "h5" => Some(5),
            "h6" => Some(6),
            _ => None,
        };

        if let Some(level) = heading_level {
            let text = node.text().collect::<String>().trim().to_string();
            if !text.is_empty() && seen.insert(text.clone()) {
                let prefix = "#".repeat(level);
                parts.push(format!("{prefix} {text}"));
            }
            return;
        }

        // Handle links → markdown
        if tag == "a" {
            if let Some(href) = node.value().attr("href") {
                let text = node.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    let resolved = resolve_url(base_url, href);
                    let line = format!("[{text}]({resolved})");
                    if seen.insert(line.clone()) {
                        parts.push(line);
                    }
                    return;
                }
            }
        }

        // Handle images → alt text
        if tag == "img" {
            if let Some(alt) = node.value().attr("alt") {
                let alt = alt.trim();
                if !alt.is_empty() {
                    let line = format!("[Image: {alt}]");
                    if seen.insert(line.clone()) {
                        parts.push(line);
                    }
                }
            }
            return;
        }

        // Handle paragraphs and list items — add line breaks
        let is_block = matches!(tag, "p" | "li" | "div" | "section" | "blockquote");

        // Collect children text
        let children: Vec<_> = node.children().collect();
        for child in &children {
            if let Some(child_ref) = scraper::element_ref::ElementRef::wrap(child.clone()) {
                Self::collect_text_recursive(
                    &child_ref,
                    document,
                    base_url,
                    depth + 1,
                    parts,
                    seen,
                );
            } else {
                // Text node
                if let Some(text_node) = child.value().as_text() {
                    let text = text_node.text.trim().to_string();
                    if !text.is_empty() && seen.insert(text.clone()) {
                        parts.push(text);
                    }
                }
            }
        }

        if is_block {
            parts.push(String::new());
        }
    }

    /// Extract all links from an HTML document, resolving relative URLs.
    pub fn extract_links(html: &str, base_url: &str) -> Vec<String> {
        let document = Html::parse_document(html);
        let mut links = Vec::new();

        if let Ok(selector) = Selector::parse("a[href]") {
            for element in document.select(&selector) {
                if let Some(href) = element.value().attr("href") {
                    let href = href.trim();
                    // Skip empty, javascript:, mailto:, tel: links
                    if href.is_empty()
                        || href.starts_with("javascript:")
                        || href.starts_with("mailto:")
                        || href.starts_with("tel:")
                        || href.starts_with("#")
                    {
                        continue;
                    }

                    let resolved = resolve_url(base_url, href);
                    // Keep only http/https
                    if resolved.starts_with("http://") || resolved.starts_with("https://") {
                        links.push(resolved);
                    }
                }
            }
        }

        links
    }
}

/// Normalize a URL by stripping the fragment and removing trailing slashes.
pub fn normalize_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url);
    without_fragment.trim_end_matches('/').to_string()
}

/// Check if a URL belongs to the same domain as the entry URL.
/// Performs exact domain match — subdomains are NOT considered the same domain.
pub fn is_same_domain(url: &str, entry_url: &str) -> bool {
    // Remove trailing slash from entry_url for consistent matching
    let entry = entry_url.trim_end_matches('/');
    url.starts_with(&format!("{}/", entry)) || url == entry
}

/// Check if the URL's path starts with the given prefix.
/// The prefix is compared against the path portion of the URL.
pub fn matches_path_prefix(url: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let path = extract_path(url);
    path.starts_with(prefix)
}

/// Extract the path portion from a URL string.
pub fn extract_path(url: &str) -> String {
    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return url.to_string();
    };

    // Split host and path — host is the first segment before /
    if let Some(slash_pos) = after_scheme.find('/') {
        after_scheme[slash_pos..].to_string()
    } else {
        // No path — the URL is just the domain
        String::new()
    }
}

/// Resolve a relative URL against a base URL.
pub fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with('/') {
        let origin = base.split('/').take(3).collect::<Vec<_>>().join("/");
        format!("{}{}", origin, relative)
    } else if relative.starts_with("../") {
        let base_trimmed = base.trim_end_matches('/');
        let parent = base_trimmed
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or(base_trimmed);
        let rest = relative.trim_start_matches("../");
        format!("{}/{}", parent, rest)
    } else {
        format!("{}/{}", base.trim_end_matches('/'), relative)
    }
}

/// Check if the page depth is within the maximum allowed depth.
pub fn is_within_depth(depth: u32, max_depth: u32) -> bool {
    depth <= max_depth
}

/// Check if we should crawl another page based on the max pages limit.
pub fn should_crawl_page(visited_count: u32, max_pages: u32) -> bool {
    visited_count < max_pages
}

/// Check if a URL has already been visited.
pub fn is_visited(url: &str, visited: &HashSet<String>) -> bool {
    visited.contains(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ── Robots.txt tests ──

    #[test]
    fn test_robots_txt_parse() {
        let robots = "User-agent: *\nDisallow: /private/\nDisallow: /admin/\nCrawl-delay: 5\n";
        let entry = WebCrawler::parse_robots_txt(robots);
        assert_eq!(entry.disallowed_paths.len(), 2);
        assert!(entry.disallowed_paths.contains(&"/private/".to_string()));
        assert!(entry.disallowed_paths.contains(&"/admin/".to_string()));
        assert_eq!(entry.crawl_delay, Some(5));
    }

    #[test]
    fn test_robots_txt_allow() {
        let disallowed = vec!["/private/".to_string(), "/admin/".to_string()];
        assert!(!WebCrawler::is_path_allowed(&disallowed, "/private/data"));
        assert!(!WebCrawler::is_path_allowed(&disallowed, "/admin"));
        assert!(WebCrawler::is_path_allowed(&disallowed, "/public"));
        assert!(WebCrawler::is_path_allowed(&disallowed, "/"));
    }

    #[test]
    fn test_robots_txt_empty() {
        let disallowed: Vec<String> = vec![];
        assert!(WebCrawler::is_path_allowed(&disallowed, "/anything"));
    }

    #[test]
    fn test_extract_origin() {
        assert_eq!(
            WebCrawler::extract_origin("https://example.com/page"),
            "https://example.com"
        );
        assert_eq!(
            WebCrawler::extract_origin("http://sub.example.com:8080/path"),
            "http://sub.example.com:8080"
        );
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            WebCrawler::extract_domain("https://example.com/page"),
            Some("example.com".to_string())
        );
        assert_eq!(
            WebCrawler::extract_domain("http://sub.example.com:8080/path"),
            Some("sub.example.com:8080".to_string())
        );
        assert_eq!(WebCrawler::extract_domain("not-a-url"), None);
    }

    #[test]
    fn test_content_extractor_empty_html() {
        let result = ContentExtractor::extract("", "https://example.com");
        assert!(result.text.is_empty() || result.text.trim().is_empty());
        assert!(result.title.is_none());
    }

    #[test]
    fn test_content_extractor_title() {
        let html = "<html><head><title>Test Page</title></head><body><p>Hello</p></body></html>";
        let result = ContentExtractor::extract(html, "https://example.com");
        assert_eq!(result.title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_content_extractor_strips_scripts() {
        let html = "<html><body><p>Hello</p><script>alert('bad')</script></body></html>";
        let result = ContentExtractor::extract(html, "https://example.com");
        assert!(!result.text.contains("alert"));
        assert!(result.text.contains("Hello"));
    }

    #[test]
    fn test_content_extractor_headings() {
        let html = "<html><body><h1>Title</h1><h2>Section</h2><p>Text</p></body></html>";
        let result = ContentExtractor::extract(html, "https://example.com");
        assert!(result.text.contains("# Title"));
        assert!(result.text.contains("## Section"));
    }

    #[test]
    fn test_extract_links_absolute() {
        let html = r#"<a href="https://example.com/page">Link</a><a href="/relative">Rel</a>"#;
        let links = ContentExtractor::extract_links(html, "https://example.com/base");
        assert!(links.contains(&"https://example.com/page".to_string()));
        assert!(links.contains(&"https://example.com/relative".to_string()));
    }

    #[test]
    fn test_extract_links_skips_non_http() {
        let html = r##"
            <a href="javascript:void(0)">JS</a>
            <a href="mailto:test@example.com">Mail</a>
            <a href="#section">Anchor</a>
            <a href="https://valid.com">Valid</a>
        "##;
        let links = ContentExtractor::extract_links(html, "https://example.com");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "https://valid.com");
    }

    #[test]
    fn test_extract_links_uses_base_url() {
        let html = r#"<a href="page.html">Page</a>"#;
        let links = ContentExtractor::extract_links(html, "https://example.com/docs/");
        assert_eq!(links[0], "https://example.com/docs/page.html");
    }

    // ── URL utility tests ──

    #[test]
    fn test_normalize_url_strips_fragment() {
        let url = "https://example.com/page#section";
        let normalized = normalize_url(url);
        assert_eq!(normalized, "https://example.com/page");
    }

    #[test]
    fn test_normalize_url_removes_trailing_slash() {
        let url = "https://example.com/page/";
        let normalized = normalize_url(url);
        assert_eq!(normalized, "https://example.com/page");
    }

    #[test]
    fn test_same_domain_enforcement() {
        assert!(is_same_domain(
            "https://example.com/page",
            "https://example.com"
        ));
        assert!(is_same_domain(
            "https://example.com/docs/guide",
            "https://example.com"
        ));
        assert!(!is_same_domain(
            "https://other.com/page",
            "https://example.com"
        ));
        assert!(!is_same_domain(
            "https://sub.example.com/page",
            "https://example.com"
        ));
    }

    #[test]
    fn test_path_prefix_filtering() {
        let prefix = "/docs";
        assert!(matches_path_prefix(
            "https://example.com/docs/guide",
            prefix
        ));
        assert!(matches_path_prefix("https://example.com/docs", prefix));
        assert!(matches_path_prefix(
            "https://example.com/docs/api/v1",
            prefix
        ));
        assert!(!matches_path_prefix(
            "https://example.com/blog/post",
            prefix
        ));
        assert!(!matches_path_prefix("https://example.com/", prefix));
    }

    #[test]
    fn test_depth_limit() {
        assert!(is_within_depth(0, 3));
        assert!(is_within_depth(3, 3));
        assert!(!is_within_depth(4, 3));
        assert!(!is_within_depth(10, 3));
    }

    #[test]
    fn test_max_pages_limit() {
        assert!(should_crawl_page(0, 100));
        assert!(should_crawl_page(99, 100));
        assert!(!should_crawl_page(100, 100));
        assert!(!should_crawl_page(200, 100));
    }

    #[test]
    fn test_url_deduplication() {
        let mut visited = HashSet::new();
        visited.insert("https://example.com/page".to_string());

        assert!(is_visited("https://example.com/page", &visited));
        assert!(!is_visited("https://example.com/other", &visited));
    }

    #[test]
    fn test_normalize_url_resolves_relative() {
        assert_eq!(
            resolve_url("https://example.com/docs/", "../guide.html"),
            "https://example.com/guide.html"
        );
        assert_eq!(
            resolve_url("https://example.com/docs/", "guide.html"),
            "https://example.com/docs/guide.html"
        );
    }
}
