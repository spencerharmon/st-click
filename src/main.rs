use st_sync;
use tokio;
use crossbeam_channel::*;

#[tokio::main]
async fn main() {
    let (tx, rx) = bounded(1);
    let sync_client = st_sync::client::Client::new(tx);
    tokio::spawn(async move {
	loop {
	    println!("{:?}", rx.recv().unwrap());
	}
    });
    match sync_client.start().await {
	Ok(()) => (),
	Err(message) => println!("{}", message)
    }
}
