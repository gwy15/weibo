use std::fs::File;
use std::io::Write;
use std::path::Path;

use futures::{stream, StreamExt};

use weibo::{Client, Result};

fn init() {
    use env_logger::{init_from_env, Env};
    init_from_env(Env::default().default_filter_or("info"));
    log::info!("初始化完成，当前版本: {}", env!("CARGO_PKG_VERSION"));
}

fn bytes_to_mib(bytes: usize) -> f64 {
    bytes as f64 / (1 << 20) as f64
}

#[tokio::main]
async fn main() -> Result<()> {
    init();

    // IO 交互
    print!("微博链接: ");
    std::io::stdout().flush().unwrap();
    let mut url = String::new();
    std::io::stdin().read_line(&mut url).unwrap();
    let url = url.trim();

    // 创建文件夹
    let root = Path::new("downloads");
    if !root.exists() {
        std::fs::create_dir(&root).unwrap();
    }
    // 匿名登录
    let client = Client::new().await?;

    // 获取全部图片 ID
    let pic_ids = client.get_pic_ids(url).await?;
    // 并发下载图片
    let results = stream::iter(pic_ids)
        .map(|pic_id| {
            let client = &client;
            async move {
                let (bytes, ext) = client.get_pic(&pic_id).await?;
                let path = root.join(format!("{}.{}", pic_id, ext));
                let written = File::create(path).unwrap().write(&bytes).unwrap();
                log::info!(
                    "文件 [{}.{}] 写入 {:.2} MiB",
                    pic_id,
                    ext,
                    bytes_to_mib(written)
                );
                weibo::Result::Ok(written)
            }
        })
        .buffer_unordered(18)
        .collect::<Vec<Result<_>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    // 总结
    let size_in_bytes = results.into_iter().sum::<usize>();
    log::info!("总共文件大小: {:.2} MiB", bytes_to_mib(size_in_bytes));

    Ok(())
}
