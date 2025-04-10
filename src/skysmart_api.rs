use serde::Serialize;
use serde_json::Value;
use std::error::Error;

use crate::api_constants;

pub struct SkysmartAPIClient {
    client: reqwest::Client,
    token: String,
    user_agent: String,
}

impl SkysmartAPIClient {
    pub fn new() -> Self {
        let user_agent = Self::generate_user_agent();
        Self {
            client: reqwest::Client::new(),
            token: String::new(),
            user_agent,
        }
    }

    fn generate_user_agent() -> String {
        let browsers = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.3 Safari/605.1.15",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:97.0) Gecko/20100101 Firefox/97.0",
        ];
        let idx = rand::random::<usize>() % browsers.len();
        browsers[idx].to_string()
    }

    pub async fn close(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.token.clear();
        Ok(())
    }

    async fn authenticate(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let resp = self.client
            .post(api_constants::URL_AUTH2)
            .header("Connection", "keep-alive")
            .header("Content-Type", "application/json")
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        if resp.status().is_success() {
            let json_resp: Value = resp.json().await?;
            if let Some(token) = json_resp.get("jwtToken").and_then(|t| t.as_str()) {
                self.token = token.to_string();
                Ok(())
            } else {
                Err("Token not found in response".into())
            }
        } else {
            Err(format!("Authentication failed with status: {}", resp.status()).into())
        }
    }

    async fn get_headers(&mut self) -> Result<reqwest::header::HeaderMap, Box<dyn Error + Send + Sync>> {
        if self.token.is_empty() {
            self.authenticate().await?;
        }

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Connection", "keep-alive".parse()?);
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("User-Agent", self.user_agent.parse()?);
        headers.insert("Accept", "application/json, text/plain, */*".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse()?,
        );

        Ok(headers)
    }

    pub async fn get_room(&mut self, task_hash: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        #[derive(Serialize)]
        struct Payload {
            #[serde(rename = "taskHash")]
            task_hash: String,
        }

        let payload = Payload {
            task_hash: task_hash.to_string(),
        };

        let headers = self.get_headers().await?;
        let resp = self.client
            .post(api_constants::URL_ROOM)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        if resp.status().is_success() {
            let json_resp: Value = resp.json().await?;

            if let Some(meta) = json_resp.get("meta") {
                if let Some(step_uuids) = meta.get("stepUuids") {
                    if let Some(step_uuids_array) = step_uuids.as_array() {
                        let uuids: Vec<String> = step_uuids_array
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        return Ok(uuids);
                    }
                }
            }
            Err("Failed to extract step UUIDs from response".into())
        } else {
            Err(format!("get_room failed with status: {}", resp.status()).into())
        }
    }

    pub async fn get_task_html(&mut self, uuid: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let headers = self.get_headers().await?;
        let url = format!("{}{}", api_constants::URL_STEPS, uuid);

        let resp = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        if resp.status().is_success() {
            let json_resp: Value = resp.json().await?;
            if let Some(content) = json_resp.get("content").and_then(|c| c.as_str()) {
                Ok(content.to_string())
            } else {
                Err("Content not found in response".into())
            }
        } else {
            Err(format!("get_task_html failed with status: {}", resp.status()).into())
        }
    }
}