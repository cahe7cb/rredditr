use serde::Deserialize;
use serde_json::Value;

pub const RECENT_OFFSET: f64 = 6.0 * 60.0 * 60.0;

#[derive(Deserialize, Clone)]
pub struct Submission {
    pub id: String,
    pub subreddit: String,
    pub title: String,
    pub permalink: String,
    pub created_utc: f64,
}

impl Submission {
    pub fn get_key(&self) -> String {
        format!("posts/{}", &self.id)
    }
    pub fn get_tag(&self, now: f64) -> String {
        format!("{}:{}", &self.created_utc, now)
    }
    pub fn get_url(&self) -> String {
        format!("https://www.reddit.com{}", &self.permalink)
    }
}

pub fn decode_submission_array(json: Value) -> Vec<Submission> {
    json["data"]["children"]
        .as_array()
        .expect("Failed to parse submissions from response")
        .clone()
        .into_iter()
        .map(|entry| {
            serde_json::from_value(entry["data"].clone()).expect("Failed to decode submission")
        })
        .collect()
}
