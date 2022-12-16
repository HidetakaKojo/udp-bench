use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{select, tick};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use udp_bench::util::ctrl_channel;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 3941)]
    port: u16,
    #[arg(long, default_value_t = 1)]
    thread_num: u16,
    #[arg(long, default_value_t = 0)]
    sleep_time: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let ticker = tick(Duration::from_millis(1000));
    let ctrlc_receiver = ctrl_channel()?;
    let stopper = Arc::new(AtomicBool::new(false));
    let sent_counter = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::<std::thread::JoinHandle<()>>::new();
    for _ in 0..args.thread_num {
        let host = args.host.clone();
        let port = args.port.clone();
        let arc_counter = sent_counter.clone();
        let cloned_stopper = stopper.clone();
        let ticker = tick(Duration::from_millis(100));
        let sleep_time = args.sleep_time.clone();
        let jh = thread::spawn(move || {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .connect(format!("{host}:{port}", host = host, port = port).as_str())
                .unwrap();
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
                        let _ = socket.send(&[0, 58]);
                        inner_counter += 1;
                        if sleep_time > 0 {
                            thread::sleep(Duration::from_millis(sleep_time));
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
                let v = sent_counter.load(Ordering::SeqCst);
                println!("Sent {} packets", v);
                sent_counter.fetch_sub(v, Ordering::SeqCst);
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
