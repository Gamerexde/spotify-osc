
pub struct HttpUrl<'a> {
    pub route: &'a str,
    pub params: Vec<(&'a str, &'a str)>
}

pub fn parse_url(url: &str) -> HttpUrl {
    let (path, query) = match url.split_once("?") {
        Some((path, query)) => (path, query),
        None => (url, ""),
    };

    let query_params: Vec<(&str, &str)> = query
        .split('&')
        .map(|param| {
            let mut parts = param.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            (key, value)
        })
        .collect();

    HttpUrl {
        route: path,
        params: query_params
    }
}