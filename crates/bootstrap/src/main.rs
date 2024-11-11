use bootstrap::get;

#[tokio::main]
async fn main() {
    env_logger::init();
    get().await.unwrap();
}
