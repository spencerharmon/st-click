#![feature(strict_provenance)]
#![feature(nll)]

mod output;
mod sequencer;
mod beat_values;
mod note_map;
use st_sync;
use tokio;
use std::{thread, time};

#[tokio::main]
async fn main() {
//    let sync_client = st_sync::client::Client::new();


    let o = output::Output::new();
    o.jack_output().await;

    loop {
	thread::sleep(time::Duration::from_millis(15));
    }
    // let mut suppress_err: bool = false;
    // loop {
    // 	match sync_client.recv_next_beat_frame() {
    // 	    Ok(val) => println!("{:?}", val),
    // 	    Err(message) => {
    // 		if !suppress_err {
    // 		    println!("{}", message);
    // 		}
    // 		suppress_err = true;
		    
    // 	    }
    // 	}
    // }
}
