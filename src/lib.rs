use reqwest;
use serde_json::{json, Value};

#[macro_use]
extern crate failure;

mod errors;
pub use errors::{Error, Result};

mod constant {
    pub static UA: &str = concat!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
        "AppleWebKit/537.36 (KHTML, like Gecko) ",
        "Chrome/79.0.3945.117 Safari/537.36"
    );
}

pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(constant::UA)
            .build()?;

        Ok(Self { client })
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        let tid = self.get_tid().await?;
        // let cookies = self.get_cookies(tid);
        // TODO: set cookies
        log::info!("匿名登陆成功");
        Ok(())
    }

    /// 获取匿名登陆所需要的 tid
    async fn get_tid(&self) -> Result<String> {
        const URL: &str = "https://passport.weibo.com/visitor/genvisitor";
        let data: Value = json!({
            "cb": "cb"
        });
        let request = self
            .client
            .post(URL)
            .form(&data) // application/x-www-form-urlencoded
            .build()
            .or_else(|e| {
                log::error!("Get tid build request failed: {}", e);
                Err(e)
            })?;

        let response = self.client.execute(request).await.or_else(|e| {
            log::error!("Failed to post tid generation request: {}", e);
            Err(e)
        })?;

        let body = response.text().await?;
        let data = Self::parse_body(body)?;

        log::debug!("data: {:?}", data);

        Ok("tid".into())
    }

    /// 解析 jsonp 返回为 serde_json::Value
    fn parse_body(body: String) -> Result<Value> {
        // body: window.cb && cb(<json>);
        let body = body
            .trim_start_matches("window.cb && cb(")
            .trim_end_matches(");");

        let value = serde_json::from_str(body)?;
        Ok(value)
    }
}
