#[derive(Clone)]
pub(crate) struct HttpClient {
    client: reqwest::Client,
    url: String,
}

impl HttpClient {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_owned(),
        }
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
