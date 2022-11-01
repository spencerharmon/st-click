#![feature(strict_provenance)]

mod output;
mod sequencer;
mod beat_values;
mod note_utils;
mod config;
use st_sync;
use tokio;
use std::{thread, time};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    sequence_name: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let o = output::Output::new();
    o.jack_output(cli.sequence_name).await;
}
