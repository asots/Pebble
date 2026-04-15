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

/// Parse a CSS style string and keep only properties from the safe allowlist.
fn filter_css_properties(style: &str) -> String {
    const SAFE_PROPERTIES: &[&str] = &[
        "color",
        "background-color",
        "background",
        "font-family",
        "font-size",
        "font-style",
        "font-weight",
        "font-variant",
        "text-align",
        "text-decoration",
        "text-indent",
        "text-transform",
        "line-height",
        "letter-spacing",
        "word-spacing",
        "white-space",
        "vertical-align",
        "direction",
        "margin",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "padding",
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "border",
        "border-top",
        "border-right",
        "border-bottom",
        "border-left",
        "border-color",
        "border-style",
        "border-width",
        "border-collapse",
        "border-spacing",
        "width",
        "max-width",
        "min-width",
        "height",
        "max-height",
        "min-height",
        "display",
        "overflow",
        "visibility",
        "float",
        "clear",
        "list-style",
        "list-style-type",
        "table-layout",
    ];

    style
        .split(';')
        .filter_map(|decl| {
            let decl = decl.trim();
            if decl.is_empty() {
                return None;
            }
            let colon = decl.find(':')?;
            let prop = decl[..colon].trim().to_lowercase();
            let value = decl[colon + 1..].trim().to_lowercase();
            if !SAFE_PROPERTIES.contains(&prop.as_str()) {
                return None;
            }
            // Double-check: reject values with URL or script references
            if value.contains("url(")
                || value.contains("expression(")
                || value.contains("javascript:")
                || value.contains("vbscript:")
            {
                return None;
            }
            Some(decl.to_string())
        })
        .collect::<Vec<_>>()
        .join("; ")
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

    // Filter style attributes using a CSS property allowlist
    builder.attribute_filter(|_element, attribute, value| {
        if attribute == "style" {
            let filtered = filter_css_properties(value);
            if filtered.is_empty() {
                None
            } else {
                Some(filtered.into())
            }
        } else {
            Some(value.into())
        }
    });

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
    // Off mode: no blocking at all
    if matches!(mode, PrivacyMode::Off) {
        return ImgAction::Keep;
    }

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
                PrivacyMode::LoadOnce | PrivacyMode::TrustSender(_) | PrivacyMode::Off => {
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
///
/// Walks the tag body respecting quoted attribute values so that a `>`
/// appearing inside `alt="foo>bar"` does not prematurely close the tag.
fn find_img_tag_end(html: &str, start: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut i = start;
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match (in_quote, b) {
            (Some(q), c) if c == q => in_quote = None,
            (None, b'"') | (None, b'\'') => in_quote = Some(b),
            (None, b'>') => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Extract the value of an attribute from an HTML tag string.
///
/// Walks attributes left-to-right, matching only whole attribute names (so
/// `data-src` is not confused with `src`) and honoring quoted/unquoted values.
fn extract_attr_value(tag: &str, attr: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let attr_lower = attr.to_ascii_lowercase();
    // Skip past `<tagname` — find first whitespace after `<`.
    let mut i = match tag.find(|c: char| c.is_ascii_whitespace()) {
        Some(idx) => idx,
        None => return None,
    };

    while i < bytes.len() {
        // Skip whitespace between attributes.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'>' || bytes[i] == b'/' {
            return None;
        }
        // Read attribute name.
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let name = tag[name_start..i].to_ascii_lowercase();

        // Skip whitespace before '='.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Attribute without value (boolean attribute).
        if i >= bytes.len() || bytes[i] != b'=' {
            if name == attr_lower {
                return Some(String::new());
            }
            continue;
        }
        // Consume '=' and following whitespace.
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }

        // Read value (quoted or unquoted).
        let value = if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let v_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let v = tag[v_start..i].to_string();
            if i < bytes.len() {
                i += 1; // consume closing quote
            }
            v
        } else {
            let v_start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'>'
                && bytes[i] != b'/'
            {
                i += 1;
            }
            tag[v_start..i].to_string()
        };

        if name == attr_lower {
            return Some(value);
        }
    }
    None
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

    #[test]
    fn test_blocks_css_url_exfiltration() {
        let guard = PrivacyGuard::new();
        let html = r#"<p style="background: url('https://evil.com/steal')">text</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("evil.com"));
    }

    #[test]
    fn test_blocks_css_import() {
        let guard = PrivacyGuard::new();
        let html = r#"<div style="@import url('https://evil.com/exfil.css')">text</div>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("evil.com"));
    }

    #[test]
    fn test_allows_safe_css_properties() {
        let guard = PrivacyGuard::new();
        let html = r#"<p style="color: red; font-size: 14px; margin: 10px">text</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(result.html.contains("color: red"));
        assert!(result.html.contains("font-size: 14px"));
    }

    #[test]
    fn test_blocks_position_properties() {
        let guard = PrivacyGuard::new();
        let html = r#"<div style="position: fixed; top: 0; left: 0; z-index: 9999">overlay</div>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("position"));
        assert!(!result.html.contains("z-index"));
    }

    #[test]
    fn img_tag_end_respects_quoted_gt() {
        // The alt attribute contains a '>' inside quotes. The naive parser
        // that looks for the first '>' would close the tag early, leaving
        // a stray src=".../pixel.gif" fragment in the output.
        let guard = PrivacyGuard::new();
        let html = r#"<p>Before</p><img alt="hi>there" src="https://tracking.mailchimp.com/open.gif" width="100" height="50"><p>After</p>"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        // Tracker must be detected and the src must not survive in output.
        assert!(!result.html.contains("mailchimp.com"), "tracker src leaked: {}", result.html);
        assert_eq!(result.trackers_blocked.len(), 1);
    }

    #[test]
    fn extract_attr_does_not_match_substring() {
        // `data-src` should NOT be treated as `src`. A substring-matching
        // parser would pull the data-src value and miss the real src.
        let guard = PrivacyGuard::new();
        let html = r#"<img data-src="https://example.com/local.jpg" src="https://tracking.mailchimp.com/open.gif" width="100" height="50">"#;
        let result = guard.render_safe_html(html, &PrivacyMode::Strict);
        assert!(!result.html.contains("mailchimp.com"), "tracker leaked: {}", result.html);
        assert_eq!(result.trackers_blocked.len(), 1, "expected real src to be detected");
    }
}
