use std::collections::HashMap;

pub fn parse_frontmatter(content: &str) -> (HashMap<String, String>, String) {
    let mut frontmatter = HashMap::new();

    if !content.starts_with("---") {
        return (frontmatter, content.to_string());
    }

    let rest = &content[3..];
    let Some(end) = rest.find("\n---") else {
        return (frontmatter, content.to_string());
    };

    let fm_block = &rest[..end];
    let body = &rest[end + 4..];

    for line in fm_block.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();
            frontmatter.insert(key, value);
        }
    }

    (frontmatter, body.trim_start().to_string())
}
