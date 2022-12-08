use anyhow::Result;
use clap::Parser;
use crossbeam_channel::select;
use std::net::UdpSocket;
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

    let ctrlc_receiver = ctrl_channel()?;
    loop {
        select! {
            recv(ctrlc_receiver) -> _ => {
                println!();
                println!("Received Ctrl-C, exiting");
                break;
            }
            default => {
                let _ = socket.send(&[0, 100]);
            }
        }
    }
    Ok(())
}
