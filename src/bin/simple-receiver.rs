use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{select, tick};
use std::io;
use std::net::UdpSocket;
use std::time::Duration;
use udp_bench::util::ctrl_channel;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value_t = 3941)]
    port: u16,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let socket = UdpSocket::bind(format!("{host}:{port}", host = args.host, port = args.port))?;
    socket.set_read_timeout(Some(Duration::from_millis(10)))?;

    let ticker = tick(Duration::from_millis(1000));
    let ctrlc_receiver = ctrl_channel()?;

    let mut buf = [0; 1500];
    let mut received_count: u64 = 0;
    loop {
        select! {
            recv(ticker) -> _ => {
                println!("Received {} packets", received_count);
                received_count = 0;
            }
            recv(ctrlc_receiver) -> _ => {
                println!();
                println!("Received Ctrl-C, exiting");
                break;
            }
            default => {
                match socket.recv_from(&mut buf)  {
                    Ok(_) => {
                        received_count += 1;
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        println!("failed to receive a datagram: {}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
