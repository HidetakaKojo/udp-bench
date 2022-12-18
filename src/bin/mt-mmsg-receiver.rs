use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{select, tick};
use nix::errno::Errno;
use nix::sys::socket::{
    self, recvmmsg, socket,
    sockopt::{ReceiveTimeout, ReusePort, RcvBuf},
    AddressFamily, MsgFlags, MultiHeaders, RecvMsg, SockFlag, SockType, SockaddrIn,
};
use nix::sys::time::TimeVal;
use std::io::IoSliceMut;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use udp_bench::util::ctrl_channel;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value_t = 3941)]
    port: u16,
    #[arg(long, default_value_t = 2)]
    thread_num: u16,
}

const BATCH_NUM: usize = 32;

fn main() -> Result<()> {
    let args = Args::parse();
    let ctrlc_receiver = ctrl_channel()?;
    let ticker = tick(Duration::from_millis(1000));

    let received_counter = Arc::new(AtomicUsize::new(0));
    let stopper = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::<std::thread::JoinHandle<()>>::new();
    for _ in 0..args.thread_num {
        let host = args.host.clone();
        let port = args.port.clone();
        let arc_counter = received_counter.clone();
        let cloned_stopper = stopper.clone();
        let jh = thread::spawn(move || {
            let sock_addr =
                SockaddrIn::from_str(format!("{host}:{port}", host = host, port = port).as_str())
                    .unwrap();
            let raw_socket = socket(
                AddressFamily::Inet,
                SockType::Datagram,
                SockFlag::empty(),
                None,
            )
            .unwrap();
            socket::setsockopt(raw_socket, ReusePort, &true).unwrap();
            socket::setsockopt(raw_socket, ReceiveTimeout, &TimeVal::new(0, 20_000)).unwrap();
            socket::setsockopt(raw_socket, RcvBuf, &(67108864 as usize)).unwrap();
            nix::sys::socket::bind(raw_socket, &sock_addr).unwrap();
            let ticker = tick(Duration::from_millis(100));
            let mut msgs = std::collections::LinkedList::new();
            let mut buf = [[0u8; 1500]; BATCH_NUM];
            msgs.extend(buf.iter_mut().map(|b| [IoSliceMut::new(&mut b[..])]));
            let mut data = MultiHeaders::<SockaddrIn>::preallocate(msgs.len(), None);
            let mut inner_counter: usize = 0;

            loop {
                if cloned_stopper.load(Ordering::Relaxed) {
                    break;
                }
                select! {
                    recv(ticker) -> _ => {
                        arc_counter.fetch_add(inner_counter, Ordering::SeqCst);
                        inner_counter = 0;
                    }
                    default => {
                        match recvmmsg(raw_socket, &mut data, msgs.iter(), MsgFlags::empty(), None) {
                            Ok(res) => {
                                let responses: Vec<RecvMsg<SockaddrIn>> = res.collect();
                                inner_counter += responses.len();
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
        });
        handles.push(jh);
    }

    loop {
        select! {
            recv(ticker) -> _ => {
                let v = received_counter.load(Ordering::SeqCst);
                println!("Received {} packets", v);
                received_counter.fetch_sub(v, Ordering::SeqCst);
            }
            recv(ctrlc_receiver) -> _ => {
                println!();
                println!("Received Ctrl-C, exiting");
                stopper.store(true, Ordering::Relaxed);
                break;
            }
            default => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    for jh in handles.into_iter() {
        jh.join().unwrap();
    }

    Ok(())
}
