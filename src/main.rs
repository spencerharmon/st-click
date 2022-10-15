use st_sync;
use tokio;

#[tokio::main]
async fn main() {
    let sync_client = st_sync::client::Client::new();
    match sync_client.start().await {
	Ok(()) => (),
	Err(message) => println!("{}", message)
    }
}
