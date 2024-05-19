use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;

#[derive(Debug)]
pub struct SimpleFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl SimpleFuture {
    pub fn new(time: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
            start_time: Instant::now(),
        }));

        let thread_shared_state = shared_state.clone();
        // tokio::spawn
        thread::spawn(move || {
            println!("Starting a background thread");

            thread::sleep(time); // this would block a scheduler in tokio::spawn
            // tokio::time::sleep(time).await; // this would not block a scheduler

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
    start_time: Instant,
}

impl Future for SimpleFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        println!("Polling the future");

        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            let taken_time = shared_state.start_time.elapsed().as_secs();

            println!("Future is already completed, took {} seconds", taken_time);

            Poll::Ready(())
        } else {
            println!("Future is not completed yet");

            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn main() -> Result<(), Error> {
    println!("03 Simple Future");

    let runtime = Builder::new_multi_thread()
        .worker_threads(1) // specify the number of worker threads
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let future1 = SimpleFuture::new(Duration::from_secs(5));
        let future2 = SimpleFuture::new(Duration::from_secs(10));

        println!("Future1: {:?}", future1);
        println!("Future2: {:?}", future2);

        let res = tokio::join!(future1, future2);

        println!("Future returned : {:?}", res);
    });

    Ok(())
}

