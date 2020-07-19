use reqwest;
use serde::de::DeserializeOwned;
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

#[doc(hidden)]
mod response {
    use crate::{Error, Result};
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    pub struct Api<T> {
        pub msg: String,
        pub retcode: i64,
        pub data: T,
    }

    impl<T> Api<T> {
        pub fn result(self) -> Result<T> {
            if self.retcode != 20_000_000 {
                Err(Error::Api(self.msg))
            } else {
                Ok(self.data)
            }
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct GenVisitor {
        pub new_tid: bool,
        pub tid: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Visitor {
        pub sub: String,
        pub subp: String,
    }
}

pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(constant::UA)
            .cookie_store(true)
            .build()?;

        Ok(Self { client })
    }

    /// 匿名登录
    pub async fn authenticate(&mut self) -> Result<()> {
        let tid = self.get_tid().await?;
        self.get_cookies(tid).await?;
        log::info!("匿名登陆成功");
        Ok(())
    }

    /// 获取匿名登陆所需要的 tid
    async fn get_tid(&self) -> Result<String> {
        const URL: &str = "https://passport.weibo.com/visitor/genvisitor";
        let data: Value = json!({
            "cb": "cb"
        });
        // 组建请求
        let request = self
            .client
            .post(URL)
            .form(&data) // application/x-www-form-urlencoded
            .build()
            .or_else(|e| {
                log::error!("Get tid build request failed: {}", e);
                Err(e)
            })?;

        // 发送请求
        let resp = self.client.execute(request).await.or_else(|e| {
            log::error!("Failed to post tid generation request: {}", e);
            Err(e)
        })?;

        // 解析
        let body = resp.text().await?;
        let data: response::GenVisitor = Self::parse_api_body(body)?;

        log::debug!("data: {:?}", data);

        Ok(data.tid)
    }

    async fn get_cookies(&self, tid: String) -> Result<()> {
        const URL: &str = "https://passport.weibo.com/visitor/visitor";

        let query = json!({
            "a": "incarnate",
            "t": tid,
            "w": 2,
            "c": "095",
            "gc": "",
            "cb": "cb",
            "from": "weibo",
            "_rand": rand::random::<f64>()
        });

        let request = self.client.get(URL).query(&query).build()?;
        let resp = self.client.execute(request).await?;

        // 读取 body，自动保存 cookie
        let body = resp.text().await?;
        let _data: response::Visitor = Self::parse_api_body(body)?;

        Ok(())
    }

    /// 解析 jsonp 返回为 T，内部处理 retcode
    fn parse_api_body<T>(body: String) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // body: window.cb && cb(<json>);
        let body = body
            .trim_start_matches("window.cb && cb(")
            .trim_end_matches(");");

        let value: response::Api<T> = serde_json::from_str(body)?;
        value.result()
    }
}
