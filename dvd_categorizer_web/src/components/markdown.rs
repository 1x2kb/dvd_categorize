use dioxus::prelude::*;

#[component]
pub fn Markdown(content: String) -> Element {
    let sanitized = sanitize_html(&content);
    
    rsx! {
        div {
            class: "markdown-content",
            dangerous_inner_html: "{sanitized}"
        }
    }
}

fn sanitize_html(html: &str) -> String {
    // Dangerous tags with their closing tags
    let dangerous_tag_pairs = [
        ("script", "script"),
        ("iframe", "iframe"),
        ("object", "object"),
        ("embed", "embed"),
        ("style", "style"),
        ("form", "form"),
    ];
    
    // Self-closing dangerous tags
    let dangerous_self_closing = ["<link", "<input", "<button"];
    
    let mut sanitized = html.to_string();
    
    // Remove paired tags with content
    for (open_tag, close_tag) in dangerous_tag_pairs {
        let open_pattern = format!("<{}", open_tag);
        let close_pattern = format!("</{}>", close_tag);
        
        loop {
            let sanitized_lower = sanitized.to_lowercase();
            if let Some(start) = sanitized_lower.find(&open_pattern.to_lowercase()) {
                // Find end of opening tag
                if let Some(open_end) = sanitized[start..].find('>') {
                    // Find closing tag
                    if let Some(close_start) = sanitized_lower[start + open_end..].find(&close_pattern.to_lowercase()) {
                        let close_end = close_start + close_pattern.len();
                        sanitized.replace_range(start..start + open_end + close_end + 1, "");
                    } else {
                        // No closing tag found, just remove opening tag
                        sanitized.replace_range(start..start + open_end + 1, "");
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    // Remove self-closing tags
    for tag in dangerous_self_closing {
        let tag_lower = tag.to_lowercase();
        while let Some(start) = sanitized.to_lowercase().find(&tag_lower) {
            if let Some(end) = sanitized[start..].find('>') {
                sanitized.replace_range(start..start + end + 1, "");
            } else {
                break;
            }
        }
    }
    
    // Remove onclick, onerror, onload and other event handlers
    let event_attrs = [
        "onclick", "onerror", "onload", "onmouseover", "onmouseout",
        "onfocus", "onblur", "onchange", "onsubmit", "onkeydown",
        "onkeyup", "onkeypress",
    ];
    
    for attr in event_attrs {
        while let Some(start) = sanitized.to_lowercase().find(attr) {
            if let Some(quote_end) = sanitized[start..].find(|c| c == '"' || c == '\'') {
                let quote_char = sanitized.chars().nth(start + quote_end).unwrap();
                if let Some(attr_end) = sanitized[start + quote_end + 1..].find(quote_char) {
                    sanitized.replace_range(start..start + quote_end + attr_end + 2, "");
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    sanitized
}
