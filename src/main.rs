fn init() {
    use env_logger::{init_from_env, Env};
    init_from_env(Env::default().default_filter_or("info"));
    log::info!("初始化完成，当前版本: {}", env!("CARGO_PKG_VERSION"));
}

#[tokio::main]
async fn main() -> weibo::Result<()> {
    init();

    let mut _client = weibo::Client::new().await?;
    Ok(())
}
