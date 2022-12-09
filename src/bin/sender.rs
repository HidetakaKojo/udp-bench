use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{select, tick};
use std::net::UdpSocket;
use std::time::Duration;
use udp_bench::util::ctrl_channel;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 3941)]
    port: u16,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(format!("{host}:{port}", host = args.host, port = args.port))?;

    let ticker = tick(Duration::from_millis(1000));
    let ctrlc_receiver = ctrl_channel()?;
    let mut sent_count: u64 = 0;
    loop {
        select! {
            recv(ticker) -> _ => {
                println!("Sent {} packets", sent_count);
                sent_count = 0;
            }
            recv(ctrlc_receiver) -> _ => {
                println!();
                println!("Received Ctrl-C, exiting");
                break;
            }
            default => {
                let _ = socket.send(&[0, 100]);
                sent_count += 1;
            }
        }
    }
    Ok(())
}
