use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use event_listener::Event;

#[derive(Debug)]
struct Channel<T> {
    closed: AtomicBool,
    events_recv: Event,
    events_queue_is_empty: Event,
    queue: Mutex<VecDeque<T>>,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Self {
            closed: AtomicBool::default(),
            events_recv: Event::default(),
            events_queue_is_empty: Event::default(),
            queue: Mutex::default(),
        }
    }
}

impl<T> Channel<T> {
    /// If channel is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, payload: T) {
        if self.channel.is_closed() {
            return;
        }

        self.channel
            .queue
            .lock()
            .expect("POISON")
            .push_back(payload);

        self.channel.events_recv.notify(usize::MAX);
    }

    pub fn close(&self) {
        self.channel.closed.store(true, Ordering::SeqCst);
    }

    pub fn queue_drained(&self) {
        loop {
            let mut listener = self.channel.events_queue_is_empty.listen();
            let lis = listener.as_mut();

            if self.channel.queue.lock().expect("POISON").is_empty() {
                break;
            }

            lis.wait();
        }
    }

    pub fn close_notify(&self) {
        self.close();
        self.channel.events_recv.notify(usize::MAX);
    }
}

#[derive(Debug, Clone)]
pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Receiver<T> {
    pub fn recv_wait(&self) -> Option<T> {
        loop {
            if self.channel.is_closed() {
                return None;
            }

            let mut listener = self.channel.events_recv.listen();
            let lis = listener.as_mut();
            let mut queue = self.channel.queue.lock().unwrap();

            if let Some(payload) = queue.pop_front() {
                return Some(payload);
            } else {
                self.channel.events_queue_is_empty.notify(usize::MAX);
            }

            drop(queue);

            lis.wait();
        }
    }
}

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Channel::new());

    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}

#[cfg(test)]
#[path = "./__tests__/channel.spec.rs"]
mod tests;
