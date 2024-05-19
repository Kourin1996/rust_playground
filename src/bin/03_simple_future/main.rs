use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct SimpleFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl SimpleFuture {
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            println!("Starting a background thread");

            thread::sleep(Duration::from_secs(5));

            println!("Background thread completing the future");

            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;

            shared_state.waker.take().and_then(|waker| {
                waker.wake();
                Some(())
            });
        });

        SimpleFuture {
            shared_state,
        }
    }
}

#[derive(Debug)]
struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl Future for SimpleFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        println!("Polling the future");

        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            println!("Future is already completed");

            Poll::Ready(())
        } else {
            println!("Future is not completed yet");

            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("03 Simple Future");

    let future = SimpleFuture::new();

    println!("Future: {:?}", future);

    let res = future.await;

    println!("Future returned : {:?}", res);

    Ok(())
}

