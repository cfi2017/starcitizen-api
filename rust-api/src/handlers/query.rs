use std::collections::HashMap;

/// Parsed query parameters from the request URL.
#[derive(Debug, Default)]
pub struct QueryParams {
    pub page: usize,
    pub per_page: usize,
    pub filters: HashMap<String, String>,
    pub sort: Option<String>,
    pub search: Option<String>,
    pub locale: Option<String>,
    pub include: Vec<String>,
}

impl QueryParams {
    pub fn from_query(query: &str) -> Self {
        let mut params = QueryParams {
            page: 1,
            per_page: 30,
            ..Default::default()
        };

        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, urldecode(v)),
                None => (pair, String::new()),
            };

            match key {
                "page[number]" => {
                    if let Ok(n) = value.parse::<usize>() {
                        params.page = n.max(1);
                    }
                }
                "page[size]" => {
                    if let Ok(n) = value.parse::<usize>() {
                        params.per_page = n.clamp(1, 200);
                    }
                }
                "sort" => {
                    params.sort = Some(value);
                }
                "locale" => {
                    params.locale = Some(value);
                }
                "include" => {
                    params.include = value.split(',').map(|s| s.trim().to_string()).collect();
                }
                "query" => {
                    params.search = Some(value);
                }
                _ => {
                    // Parse filter[field] syntax
                    if let Some(field) = key
                        .strip_prefix("filter[")
                        .and_then(|s| s.strip_suffix(']'))
                    {
                        params.filters.insert(field.to_string(), value);
                    }
                }
            }
        }

        params
    }
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();

    while let Some(b) = chars.next() {
        match b {
            b'+' => result.push(' '),
            b'%' => {
                let hi = chars.next().and_then(|c| hex_val(c));
                let lo = chars.next().and_then(|c| hex_val(c));
                if let (Some(h), Some(l)) = (hi, lo) {
                    result.push((h << 4 | l) as char);
                } else {
                    result.push('%');
                }
            }
            _ => result.push(b as char),
        }
    }

    result
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Sort items by a field path (supports `-field` for descending).
/// Returns indices into the slice in sorted order.
pub fn sort_by_json_path<T, F>(items: &[T], sort_spec: &str, get_data: F) -> Vec<usize>
where
    F: Fn(&T) -> &serde_json::Value,
{
    let descending = sort_spec.starts_with('-');
    let field_path = if descending { &sort_spec[1..] } else { sort_spec };

    // Convert dot notation to JSON pointer
    let pointer = format!("/{}", field_path.replace('.', "/"));

    let mut indices: Vec<usize> = (0..items.len()).collect();

    indices.sort_by(|&a, &b| {
        let val_a = get_data(&items[a]).pointer(&pointer);
        let val_b = get_data(&items[b]).pointer(&pointer);

        let cmp = compare_json_values(val_a, val_b);

        if descending { cmp.reverse() } else { cmp }
    });

    indices
}

fn compare_json_values(
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => {
            // Compare by type
            match (a, b) {
                (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                    let fa = a.as_f64().unwrap_or(0.0);
                    let fb = b.as_f64().unwrap_or(0.0);
                    fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                }
                (serde_json::Value::String(a), serde_json::Value::String(b)) => {
                    a.to_lowercase().cmp(&b.to_lowercase())
                }
                (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a.cmp(b),
                _ => std::cmp::Ordering::Equal,
            }
        }
    }
}

/// Case-insensitive substring match for search.
pub fn matches_search(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
