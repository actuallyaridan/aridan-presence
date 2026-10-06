// Talking to the Worker at presence.aridan.net.

use crate::activity::Activity;
use crate::config::Config;
use serde::Serialize;
use std::time::Duration;

pub struct Server {
    client: reqwest::Client,
    device_url: String,
    token: String,
}

// The body of a report: { "activities": [...] }
#[derive(Serialize)]
struct Report<'a> {
    activities: &'a [Activity],
}

impl Server {
    pub fn new(config: &Config) -> Server {
        // A report that hangs would hold up the next one, so it gives up
        // after ten seconds and the next tick tries again.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("the HTTP client should always build");

        Server {
            client: client,
            device_url: format!("{}/devices/{}", config.server, config.device),
            token: config.token.clone(),
        }
    }

    pub async fn report(&self, activities: &[Activity]) -> Result<(), String> {
        let body = Report { activities: activities };

        let request = self
            .client
            .put(&self.device_url)
            .bearer_auth(&self.token)
            .json(&body);

        send(request).await
    }

    // Called on the way out, so the site stops showing this computer's song
    // at once instead of 90 seconds later.
    pub async fn clear(&self) -> Result<(), String> {
        let request = self.client.delete(&self.device_url).bearer_auth(&self.token);

        send(request).await
    }
}

async fn send(request: reqwest::RequestBuilder) -> Result<(), String> {
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => return Err(format!("could not reach the server: {}", error)),
    };

    let status = response.status();

    if status.is_success() {
        return Ok(());
    }

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(String::from("the server did not accept the token"));
    }

    let text = response.text().await.unwrap_or_default();
    Err(format!("the server answered {} {}", status, text))
}
