#[derive(Clone)]
pub(crate) struct HttpClient {
    pub client: reqwest::Client,
    pub url: String,
}

impl HttpClient {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_owned(),
        }
    }
}
