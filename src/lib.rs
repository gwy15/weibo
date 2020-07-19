use bytes::Bytes;
use reqwest;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

#[macro_use]
extern crate failure;

mod errors;
pub use errors::{Error, Result};

mod constant {
    use lazy_static::lazy_static;
    use regex::Regex;

    pub static UA: &str = concat!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
        "AppleWebKit/537.36 (KHTML, like Gecko) ",
        "Chrome/79.0.3945.117 Safari/537.36"
    );
    lazy_static! {
        pub static ref PIC_ID_REGEX: Regex = Regex::new("pic_ids=([^&]+)&").unwrap();
    }
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
    inner_client: reqwest::Client,
}

impl Client {
    /// 返回一个已经匿名登陆的客户端
    pub async fn new() -> Result<Self> {
        let inner_client = reqwest::ClientBuilder::new()
            .user_agent(constant::UA)
            .cookie_store(true)
            .build()?;

        let mut client = Self { inner_client };
        // 完成匿名登录
        client.authenticate().await?;

        Ok(client)
    }

    /// 匿名登录
    async fn authenticate(&mut self) -> Result<()> {
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
            .inner_client
            .post(URL)
            .form(&data) // application/x-www-form-urlencoded
            .build()
            .or_else(|e| {
                log::error!("Get tid build request failed: {}", e);
                Err(e)
            })?;

        // 发送请求
        let resp = self.inner_client.execute(request).await.or_else(|e| {
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

        let request = self.inner_client.get(URL).query(&query).build()?;
        let resp = self.inner_client.execute(request).await?;

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

    /// 获取微博链接中的全部图片 url
    pub async fn get_pic_ids(&self, url: &str) -> Result<Vec<String>> {
        let request = self.inner_client.get(url).build()?;
        let resp = self.inner_client.execute(request).await?;
        let html = resp.text().await?;

        let regex = &constant::PIC_ID_REGEX;

        let pic_ids = regex
            .captures(&html)
            .map(|cap| cap[1].split(",").map(|s| s.to_owned()).collect())
            .unwrap_or_else(|| vec![]);

        Ok(pic_ids)
    }

    /// 下载图片，返回二进制图片及其扩展名
    pub async fn get_pic(&self, pic_id: &str) -> Result<(Bytes, String)> {
        let url = format!("https://wx4.sinaimg.cn/large/{}.jpg", pic_id);
        let request = self.inner_client.get(&url).build()?;
        let resp = self.inner_client.execute(request).await?;

        let content_type = resp
            .headers()
            .get("Content-Type")
            .map(|v| v.to_str().unwrap().replace("image/", "").to_lowercase())
            .unwrap_or_else(|| "jpg".to_owned());
        let ext = match content_type.as_str() {
            "jpeg" => "jpg",
            "jpg" | "png" | "gif" | "bmp" | "webp" => &content_type,
            _ => panic!("未知图片扩展名：{}", content_type),
        };

        let bytes = resp.bytes().await?;
        Ok((bytes, ext.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// 测试匿名登录和拉取图片 id
    #[tokio::test]
    async fn test_integration() {
        // 测试登录
        let client = Client::new().await.unwrap();

        // 测试获取图片
        const URL: &str = "https://weibo.com/2656274875/JbTu3a9Td?filter=hot&root_comment_id=0";
        let pic_ids = client.get_pic_ids(URL).await.unwrap();
        println!("{:#?}", pic_ids);
        assert_eq!(pic_ids.len(), 9);
        assert_eq!(
            pic_ids,
            vec![
                "9e5389bbly1ggw2ssj9gzj20u00k0dhs",
                "9e5389bbly1ggw2ssjpv5j20u00jxacp",
                "9e5389bbly1ggw2ssjl0gj20u0104goh",
                "9e5389bbly1ggw2ssmxopj20u00kb0vg",
                "9e5389bbly1ggw2ssjo2hj20lm0sitbi",
                "9e5389bbly1ggw2sskebaj20u017un1o",
                "9e5389bbly1ggw2sslo9hj20u00k0jsh",
                "9e5389bbly1ggw2ssmeuwj20u011qad3",
                "9e5389bbly1ggw2ssnohnj20u0190wi7",
            ]
        );

        // 测试下载图片
        let (bytes, ext) = client
            .get_pic("9e5389bbly1ggw2ssj9gzj20u00k0dhs")
            .await
            .unwrap();
        assert_eq!(ext, "jpg");
        assert_eq!(bytes.len(), 82_803);
    }
}
