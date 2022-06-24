#[macro_use]
extern crate serde;

use std::time::Duration;
use anyhow::Result;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};

const API_ROOT: &'static str = "https://openpagerank.com/api/v1.0/getPageRank";

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Response {
    pub status_code: u16,
    pub error: String,
    pub page_rank_integer: u32,
    pub page_rank_decimal: f32,
    pub rank: Option<String>,
    pub domain: String
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct PageRank {
    pub status_code: u16,
    pub response: Vec<Response>,
    pub last_updated: String
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct PageRankFirst {
    pub status_code: u16,
    pub response: Response,
    pub last_updated: String
}

// used in url feature
impl TryInto<PageRankFirst> for PageRank {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PageRankFirst, Self::Error> {
        match self.response.get(0) {
            Some(response) => Ok(PageRankFirst {
                status_code: self.status_code,
                response: response.clone(),
                last_updated: self.last_updated,
            }),
            None => return Err(anyhow::anyhow!("no response found"))
        }
    }
} 

impl PageRank {
    pub async fn rank<T> (domains: Vec<T>, key: &str, timeout: Duration) -> Result<Self> 
    where T: AsRef<str>
    {
        let mut headers = HeaderMap::new();
        headers.insert("API-OPR", HeaderValue::from_str(key)?);

        let client = Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(true)
            .build()?;

        let domains = domains.into_iter().map(|x| Self::remove_trailing_slash(x.as_ref())).collect::<Vec<_>>();
        let query = domains.into_iter().map(|x| ("domains[]", x)).collect::<Vec<_>>();

        let rank = client.get(API_ROOT)
            .query(&query)
            .timeout(timeout)
            .send()
            .await?
            .json::<Self>()
            .await?;

        Ok(rank)
    }

    // open rank api need no trailing slash on url
    fn remove_trailing_slash<T>(s: T) -> String
    where T: ToString
    {
        let mut s = s.to_string();
        while s.ends_with('/') { s.pop(); }
        return s
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn last_updated(&self) -> String {
        self.last_updated.clone()
    }

    pub fn response(&self) -> Vec<Response> {
        self.response.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_url() {
        let url = url::Url::parse("https://monitorapp.com").unwrap();
        let rank = aw!(PageRank::rank(vec![url], &api_key(), Duration::from_secs(10))).unwrap();
        assert!(rank.response[0].rank.is_some());
        assert_eq!(rank.response[0].domain, "monitorapp.com");
    }

    #[test]
    fn test_domain() {
        let rank =aw!(PageRank::rank(vec!["monitorapp.com"], &api_key(), Duration::from_secs(10))).unwrap();
        assert!(rank.response[0].rank.is_some());
        assert_eq!(rank.response[0].domain, "monitorapp.com");
    }

    #[test]
    fn test_invalid_url() {
        let rank =aw!(PageRank::rank(vec!["https://invalid-monitorapp.com"], &api_key(), Duration::from_secs(10))).unwrap();
        assert_eq!(rank.response[0].rank, None);
    }

    fn api_key() -> String {
        "kc8kgoc00oo00ggskksc00kgo0o4o04swkc0cs88".to_string()
    }
}