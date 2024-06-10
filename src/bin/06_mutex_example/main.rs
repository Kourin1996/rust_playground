use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    println!("Hello, world!");

    // No.1
    let n = Mutex::new(0);

    thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                let mut guard = n.lock().unwrap();
                for _ in 0..100 {
                    *guard += 1;
                }
                drop(guard);
                thread::sleep(Duration::from_millis(100));
            });
        }
    });

    // unwrap mutex guard
    let res = n.into_inner().unwrap();
    println!("n = {:?}", res);

    // No.2
    println!("\nNo.2\n");
    let queue = Mutex::new(VecDeque::new());

    thread::scope(|s| {
        let t = s.spawn(|| {
            let mut count = 0;
            loop {
                let item = queue.lock().unwrap().pop_front();
                println!("pop_front() = {:?}", item);

                if let Some(item) = item {
                    dbg!(item);
                    count += 1;
                    if count >= 10 {
                        break;
                    }
                } else {
                    println!("queue is empty, calling park()");
                    thread::park(); // wait for unpark to be called
                    println!("unparked");
                }
            }
        });

        for i in 0..10 {
            queue.lock().unwrap().push_back(i);
            println!("push_back({})", i);
            t.thread().unpark();
            thread::sleep(Duration::from_millis(1000));
        }
    });

    // No.3
    println!("\nNo.3\n");
    let queue = Mutex::new(VecDeque::new());
    let not_empty = Condvar::new();

    thread::scope(|s| {
        s.spawn(|| {
            let mut count = 0;
            loop {
                let mut q = queue.lock().unwrap();
                let item = loop {
                    if let Some(item) = q.pop_front() {
                        break item;
                    } else {
                        q = not_empty.wait(q).unwrap();
                    }
                };
                drop(q);
                dbg!(item);

                count += 1;
                if count >= 10 {
                    break;
                }
            }
        });

        for i in 0..10 {
            queue.lock().unwrap().push_back(i);
            not_empty.notify_one();
            thread::sleep(Duration::from_millis(1000));
        }
    });
}
