use crate::constants::USER_AGENT;
use reqwest;
use tokio::time::Duration;

pub fn create_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse().unwrap());
            headers.insert("Accept-Language", "en-US,en;q=0.9,fr;q=0.8".parse().unwrap());
            headers.insert("Accept-Encoding", "gzip, deflate".parse().unwrap());
            headers.insert("DNT", "1".parse().unwrap());
            headers.insert("Connection", "keep-alive".parse().unwrap());
            headers.insert("Upgrade-Insecure-Requests", "1".parse().unwrap());
            headers
        })
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))
}

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn get_with_headers(
        &self,
        url: &str,
        headers: &Option<std::collections::HashMap<String, String>>,
    ) -> Result<String, String> {
        let mut request = self.client.get(url);

        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                request = request.header(key, value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("HTTP GET failed for '{}': {}", url, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP GET failed with status {}: {}",
                response.status(),
                url
            ));
        }

        response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))
    }
}