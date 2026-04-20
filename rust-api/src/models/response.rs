use serde::Serialize;

/// Standard paginated API response matching the Laravel JSON:API format.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub links: PaginationLinks,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationLinks {
    pub first: String,
    pub last: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub current_page: usize,
    pub from: usize,
    pub last_page: usize,
    pub per_page: usize,
    pub to: usize,
    pub total: usize,
    pub processed_at: String,
}

/// Single-item API response.
#[derive(Debug, Serialize)]
pub struct SingleResponse<T: Serialize> {
    pub data: T,
    pub meta: SingleMeta,
}

#[derive(Debug, Serialize)]
pub struct SingleMeta {
    pub processed_at: String,
}

impl PaginationMeta {
    pub fn new(page: usize, per_page: usize, total: usize) -> Self {
        let last_page = if total == 0 { 1 } else { (total + per_page - 1) / per_page };
        let from = if total == 0 { 0 } else { (page - 1) * per_page + 1 };
        let to = (from + per_page - 1).min(total);

        Self {
            current_page: page,
            from,
            last_page,
            per_page,
            to,
            total,
            processed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl PaginationLinks {
    pub fn new(base_url: &str, page: usize, per_page: usize, total: usize) -> Self {
        let last_page = if total == 0 { 1 } else { (total + per_page - 1) / per_page };

        Self {
            first: format!("{base_url}?page[number]=1&page[size]={per_page}"),
            last: format!("{base_url}?page[number]={last_page}&page[size]={per_page}"),
            prev: if page > 1 {
                Some(format!("{base_url}?page[number]={}&page[size]={per_page}", page - 1))
            } else {
                None
            },
            next: if page < last_page {
                Some(format!("{base_url}?page[number]={}&page[size]={per_page}", page + 1))
            } else {
                None
            },
        }
    }
}

pub fn paginate<T: Serialize + Clone>(
    items: &[T],
    page: usize,
    per_page: usize,
    base_url: &str,
) -> PaginatedResponse<T> {
    let total = items.len();
    let page = page.max(1);
    let per_page = per_page.clamp(1, 200);
    let start = (page - 1) * per_page;

    let data = if start < total {
        items[start..(start + per_page).min(total)].to_vec()
    } else {
        vec![]
    };

    PaginatedResponse {
        data,
        links: PaginationLinks::new(base_url, page, per_page, total),
        meta: PaginationMeta::new(page, per_page, total),
    }
}

pub fn single<T: Serialize>(data: T) -> SingleResponse<T> {
    SingleResponse {
        data,
        meta: SingleMeta {
            processed_at: chrono::Utc::now().to_rfc3339(),
        },
    }
}
