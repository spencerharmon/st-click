mod output;
use st_sync;
use tokio;

#[tokio::main]
async fn main() {
    let sync_client = st_sync::client::Client::new();


    let o = output::Output::new();
    tokio::task::spawn(o.jack_output());
    
    let mut suppress_err: bool = false;
    loop {
	match sync_client.recv_next_beat_frame() {
	    Ok(val) => println!("{:?}", val),
	    Err(message) => {
		if !suppress_err {
		    println!("{}", message);
		}
		suppress_err = true;
		    
	    }
	}
    }
}
