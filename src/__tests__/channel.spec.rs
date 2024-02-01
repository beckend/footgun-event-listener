use std::{sync::atomic::AtomicUsize, thread};

use super::*;
static PAYLOAD: &str = "PAYLOAD";

fn test_queue_drained() {
    let (sender, receiver) = unbounded();

    let max_loop = 10_usize;
    let counter = AtomicUsize::default();
    let counter_recv_done = Mutex::new(0_usize);

    thread::scope(|s| {
        for _ in usize::MIN..max_loop {
            s.spawn(|| {
                while let Some(x) = receiver.recv_wait() {
                    assert_eq!(x, PAYLOAD);
                    counter.fetch_add(1, Ordering::AcqRel);
                }

                let mut counter_done = counter_recv_done.lock().expect("POISON");
                *counter_done += 1;
            });
        }

        s.spawn(|| {
            for _ in usize::MIN..max_loop {
                sender.send(PAYLOAD);
            }

            sender.queue_drained();
            sender.close_notify();
        });
    });

    assert_eq!(counter.load(Ordering::Acquire), max_loop);
}

#[test]
fn test_extensive() {
    for _ in usize::MIN..1 {
        test_queue_drained();
    }
}
