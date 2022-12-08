use anyhow::Result;
use crossbeam_channel::{bounded, Receiver};

pub fn ctrl_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (tx, rx) = bounded(100);
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })?;
    Ok(rx)
}
