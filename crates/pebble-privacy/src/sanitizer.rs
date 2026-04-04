use std::collections::HashSet;

use ammonia::Builder;
use pebble_core::{PrivacyMode, RenderedHtml, TrackerInfo};

use crate::tracker::{is_known_tracker, is_tracking_pixel};

pub struct PrivacyGuard;

impl PrivacyGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn render_safe_html(&self, raw_html: &str, mode: &PrivacyMode) -> RenderedHtml {
        let mut trackers_blocked: Vec<TrackerInfo> = Vec::new();
        let mut images_blocked: u32 = 0;

        // Pre-process images before ammonia sanitization
        let preprocessed =
            preprocess_images(raw_html, mode, &mut trackers_blocked, &mut images_blocked);

        // Sanitize with ammonia
        let sanitizer = build_sanitizer(mode);
        let clean_html = sanitizer.clean(&preprocessed).to_string();

        RenderedHtml {
            html: clean_html,
            trackers_blocked,
            images_blocked,
        }
    }
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an ammonia sanitizer configured for safe email HTML rendering.
fn build_sanitizer(_mode: &PrivacyMode) -> Builder<'static> {
    let mut builder = Builder::new();

    // Allow safe tags for email HTML
    let tags: HashSet<&'static str> = [
        "a", "abbr", "b", "blockquote", "br", "code", "dd", "div", "dl", "dt", "em", "h1",
        "h2", "h3", "h4", "h5", "h6", "hr", "i", "img", "li", "ol", "p", "pre", "s", "span",
        "strong", "sub", "sup", "table", "tbody", "td", "th", "thead", "tr", "u", "ul",
        "center", "font",
    ]
    .iter()
    .copied()
    .collect();

    builder.tags(tags);

    // Configure per-tag attributes
    builder.tag_attributes(
        [
            (
                "a",
                ["href", "title", "target"].iter().copied().collect::<HashSet<_>>(),
            ),
            (
                "img",
                ["src", "alt", "width", "height", "class"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "td",
                ["colspan", "rowspan", "align", "valign"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "th",
                ["colspan", "rowspan", "align", "valign"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "table",
                ["border", "cellpadding", "cellspacing", "width", "align"]
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>(),
            ),
            (
                "font",
                ["color", "size", "face"].iter().copied().collect::<HashSet<_>>(),
            ),
            (
                "div",
                ["class", "data-src"].iter().copied().collect::<HashSet<_>>(),
            ),
            (
                "blockquote",
                ["cite"].iter().copied().collect::<HashSet<_>>(),
            ),
        ]
        .iter()
        .cloned()
        .collect(),
    );

    // Generic attributes allowed on all tags
    builder.generic_attributes(
        ["style", "class", "dir", "id"]
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
    );

    // Only allow safe URL schemes (blocks javascript:, data:, vbscript:, etc.)
    builder.url_schemes(
        ["http", "https", "mailto"]
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
    );

    // Add rel="noopener noreferrer" to all links
    builder.link_rel(Some("noopener noreferrer"));

    builder
}

/// Pre-process img tags before ammonia to handle tracking pixels and privacy modes.
fn preprocess_images(
    html: &str,
    mode: &PrivacyMode,
    trackers_blocked: &mut Vec<TrackerInfo>,
    images_blocked: &mut u32,
) -> String {
    let mut result = String::with_capacity(html.len());
    let mut pos = 0;

    while pos < html.len() {
        // Find the next <img tag
        if let Some(tag_start) = find_tag_start(html, pos, "img") {
            // Copy everything before this img tag
            result.push_str(&html[pos..tag_start]);

            // Find the end of this img tag
            if let Some(tag_end) = find_img_tag_end(html, tag_start) {
                let tag_str = &html[tag_start..tag_end];

                let src = extract_attr_value(tag_str, "src");
                let width = extract_attr_value(tag_str, "width");
                let height = extract_attr_value(tag_str, "height");

                let should_replace = process_img_tag(
                    src.as_deref(),
                    width.as_deref(),
                    height.as_deref(),
                    mode,
                    trackers_blocked,
                    images_blocked,
                );

                match should_replace {
                    ImgAction::Remove => {
                        // Skip the tag entirely (replace with nothing)
                    }
                    ImgAction::BlockedPlaceholder => {
                        let src_val = src.as_deref().unwrap_or("");
                        let escaped = html_escape(src_val);
                        result.push_str(&format!(
                            r#"<div class="blocked-image" data-src="{}">Image blocked for privacy</div>"#,
                            escaped
                        ));
                    }
                    ImgAction::Keep => {
                        result.push_str(tag_str);
                    }
                }

                pos = tag_end;
            } else {
                // Malformed tag — copy the '<' and advance
                result.push('<');
                pos = tag_start + 1;
            }
        } else {
            // No more img tags
            result.push_str(&html[pos..]);
            break;
        }
    }

    result
}

enum ImgAction {
    Remove,
    BlockedPlaceholder,
    Keep,
}

fn process_img_tag(
    src: Option<&str>,
    width: Option<&str>,
    height: Option<&str>,
    mode: &PrivacyMode,
    trackers_blocked: &mut Vec<TrackerInfo>,
    images_blocked: &mut u32,
) -> ImgAction {
    // Tracking pixels are always blocked
    if is_tracking_pixel(width, height) {
        let domain = src
            .and_then(extract_domain_from_url)
            .unwrap_or_default();
        trackers_blocked.push(TrackerInfo {
            domain,
            tracker_type: "pixel".to_string(),
        });
        return ImgAction::Remove;
    }

    // Known tracker domains are always blocked
    if let Some(src_val) = src {
        if let Some(domain) = extract_domain_from_url(src_val) {
            if is_known_tracker(&domain) {
                trackers_blocked.push(TrackerInfo {
                    domain,
                    tracker_type: "domain".to_string(),
                });
                return ImgAction::Remove;
            }
        }

        // External images depend on privacy mode
        let is_external = src_val.starts_with("http://") || src_val.starts_with("https://");
        if is_external {
            match mode {
                PrivacyMode::Strict => {
                    *images_blocked += 1;
                    return ImgAction::BlockedPlaceholder;
                }
                PrivacyMode::LoadOnce | PrivacyMode::TrustSender(_) => {
                    return ImgAction::Keep;
                }
            }
        }
    }

    ImgAction::Keep
}

/// Find the byte position where the next `<tag` starts, searching from `from`.
fn find_tag_start(html: &str, from: usize, tag: &str) -> Option<usize> {
    let search_area = &html[from..];
    let mut offset = 0;

    while offset < search_area.len() {
        if let Some(lt_pos) = search_area[offset..].find('<') {
            let abs = offset + lt_pos;
            let after_lt = abs + 1;

            // Skip closing tags
            if search_area[after_lt..].starts_with('/') {
                offset = abs + 1;
                continue;
            }

            let rest = &search_area[after_lt..];
            let rest_lower = rest.to_ascii_lowercase();
            if rest_lower.starts_with(tag) {
                let tag_len = tag.len();
                // Verify it is the full tag name (not a prefix of another tag)
                if let Some(c) = rest.chars().nth(tag_len) {
                    if c == ' ' || c == '>' || c == '/' || c == '\t' || c == '\n' || c == '\r' {
                        return Some(from + abs);
                    }
                } else {
                    // End of string after tag name — still valid
                    return Some(from + abs);
                }
            }

            offset = abs + 1;
        } else {
            break;
        }
    }

    None
}

/// Find the end position (exclusive) of an img tag starting at `start`.
fn find_img_tag_end(html: &str, start: usize) -> Option<usize> {
    let rest = &html[start..];
    // img tags can be self-closed or end with >
    rest.find('>').map(|gt_pos| start + gt_pos + 1)
}

/// Extract the value of an attribute from an HTML tag string.
fn extract_attr_value(tag: &str, attr: &str) -> Option<String> {
    let tag_lower = tag.to_ascii_lowercase();
    let search = format!("{}=", attr);

    let attr_pos = tag_lower.find(&search)?;
    let value_start = attr_pos + search.len();

    let bytes = tag.as_bytes();
    if value_start >= bytes.len() {
        return None;
    }

    if bytes[value_start] == b'"' || bytes[value_start] == b'\'' {
        let quote = bytes[value_start];
        let start = value_start + 1;
        let end = tag[start..].find(|c: char| c as u8 == quote).map(|p| p + start)?;
        Some(tag[start..end].to_string())
    } else {
        let start = value_start;
        let end = tag[start..]
            .find([' ', '>', '/'])
            .map(|p| p + start)
            .unwrap_or(tag.len());
        Some(tag[start..end].to_string())
    }
}

/// Extract the domain from a URL, stripping protocol and path.
fn extract_domain_from_url(url: &str) -> Option<String> {
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let domain = without_protocol.split('/').next().unwrap_or(without_protocol);
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

/// Escape special HTML characters for use in attribute values.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_script_tags() {
        let guard = PrivacyGuard::new();
        let html = "<p>Hello</p><script>alert('xss')</script><p>World</p>";
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("script"));
        assert!(!result.html.contains("alert"));
        assert!(result.html.contains("Hello"));
        assert!(result.html.contains("World"));
    }

    #[test]
    fn test_removes_event_handlers() {
        let guard = PrivacyGuard::new();
        let html = r#"<p onmouseover="alert(1)">Hello</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("onmouseover"));
        assert!(result.html.contains("Hello"));
    }

    #[test]
    fn test_blocks_javascript_urls() {
        let guard = PrivacyGuard::new();
        let html = r#"<a href="javascript:alert(1)">Click me</a>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("javascript:"));
        assert!(result.html.contains("Click me"));
    }

    #[test]
    fn test_removes_iframe_tags() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><iframe src="https://evil.com">content</iframe><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("iframe"));
        assert!(result.html.contains("Before"));
        assert!(result.html.contains("After"));
    }

    #[test]
    fn test_removes_style_tags() {
        let guard = PrivacyGuard::new();
        let html = "<p>Hello</p><style>body { color: red; }</style><p>World</p>";
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("<style>"));
        assert!(!result.html.contains("color: red"));
        assert!(result.html.contains("Hello"));
        assert!(result.html.contains("World"));
    }

    #[test]
    fn test_blocks_tracking_pixel() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracker.example.com/pixel.gif" width="1" height="1"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("tracker.example.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
        assert_eq!(result.trackers_blocked[0].tracker_type, "pixel");
    }

    #[test]
    fn test_blocks_known_tracker_domain() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://tracking.mailchimp.com/open.gif" width="100" height="50"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("mailchimp.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
        assert_eq!(result.trackers_blocked[0].tracker_type, "domain");
    }

    #[test]
    fn test_blocks_external_images_in_strict_mode() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://example.com/photo.jpg"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("blocked-image"));
        assert_eq!(result.images_blocked, 1);
    }

    #[test]
    fn test_allows_images_in_load_once_mode() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Hello</p><img src="https://example.com/photo.jpg"><p>World</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::LoadOnce);
        assert!(result.html.contains("https://example.com/photo.jpg"));
        assert_eq!(result.images_blocked, 0);
    }

    #[test]
    fn test_still_blocks_trackers_in_load_once_mode() {
        let guard = PrivacyGuard::new();
        let html = r#"<img src="https://tracking.mailchimp.com/open.gif" width="100" height="50">"#;
        let result = guard.render_safe_html(html, &PrivacyMode::LoadOnce);
        assert!(!result.html.contains("mailchimp.com"));
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn test_removes_svg_with_event_handlers() {
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><svg onload="alert(1)"><circle r="10"/></svg><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("onload"));
        assert!(!result.html.contains("svg"));
        assert!(result.html.contains("Before"));
        assert!(result.html.contains("After"));
    }
}
