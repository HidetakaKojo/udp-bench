use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{select, tick};
use nix::errno::Errno;
use nix::sys::socket::{recvmmsg, MsgFlags, MultiHeaders, RecvMsg, SockaddrIn};
use std::os::unix::io::AsRawFd;
use std::{io::IoSliceMut, net::UdpSocket, time::Duration};
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

    let ticker = tick(Duration::from_millis(1000));
    let ctrlc_receiver = ctrl_channel()?;

    const BATCH_NUM: usize = 10;

    let mut msgs = std::collections::LinkedList::new();
    let mut buf = [[0u8; 1500]; BATCH_NUM];
    msgs.extend(buf.iter_mut().map(|b| [IoSliceMut::new(&mut b[..])]));
    let mut data = MultiHeaders::<SockaddrIn>::preallocate(msgs.len(), None);

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
                match recvmmsg(socket.as_raw_fd(), &mut data, msgs.iter(), MsgFlags::MSG_DONTWAIT, None) {
                    Ok(res) => {
                        let responses: Vec<RecvMsg<SockaddrIn>> = res.collect();
                        received_count += responses.len() as u64;
                    }
                    Err(e) if (e == Errno::EAGAIN) || (e == Errno::EWOULDBLOCK) => {
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
