use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize};
use std::thread;
use std::time::Duration;

fn main() {
    println!("Hello, World!");

    // f1();
    // f2();
    // f3();
    f4();
}

// simple example of Atomic
fn f1() {
    println!("f1() called");

    static STOP: AtomicBool = AtomicBool::new(false);

    let background_thread = std::thread::spawn(|| {
        while !STOP.load(Relaxed) {
            println!("STOP is false");
            thread::sleep(Duration::from_secs(1));
        }
    });

    thread::sleep(Duration::from_secs(5));

    println!("Setting STOP to true");

    STOP.store(true, Relaxed);

    background_thread.join().unwrap();
}

// simple example of Atomic and park_timeout
fn f2() {
    println!("f2() called");

    let num_done = AtomicUsize::new(0);
    let main_thread = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..100 {
                let s = if i % 2 == 0 { 2 } else { 1 };
                thread::sleep(Duration::from_secs(s));
                println!("Task done, i = {}", i);
                num_done.store(i + 1, Relaxed);
                main_thread.unpark();
            }
        });

        loop {
            let n = num_done.load(Relaxed);
            if n >= 100 {
                break;
            }

            println!("Working.. {n}/100 done");
            thread::park_timeout(Duration::from_secs(1)); // sleep for up to 1 second
        }
    });
}

// simple example of sync::Once
fn f3() {
    println!("f3() called");

    {
        static INIT: std::sync::Once = std::sync::Once::new();
        thread::scope(|s| {
            for i in 0..10 {
                s.spawn(move || {
                    println!("Calling INIT i={i}");
                    INIT.call_once(|| {
                        println!("INIT is called i={i}");
                        thread::sleep(Duration::from_secs(2));
                        println!("INIT is done i={i}");
                    });
                    println!("Exiting from thread i={i}");
                });
            }
        });
    }

    println!();

    {
        static INSTANCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

        thread::scope(|s| {
            for i in 0..10 {
                s.spawn(move || {
                    println!("Calling INSTANCE i={i}");
                    let res = INSTANCE.get_or_init(|| {
                        println!("Creating instance i={i}");
                        thread::sleep(Duration::from_secs(2));
                        format!("Instance i={i}")
                    });
                    println!("Instance result i={i}, res={res}");
                });
            }
        });
    }
}

fn f4() {
    println!("f4() called");

    {
        let a = AtomicI32::new(100);
        let b = a.fetch_add(10, Relaxed);
        let c = a.load(Relaxed);

        println!("[1] a={:?}, b={}, c={}", a, b, c);
    }

    {
        let a  = AtomicI32::new(100);
        let b = a.compare_exchange(100, 200, Relaxed, Relaxed);
        let c = a.load(Relaxed);

        println!("[2] a={:?}, b={:?}, c={}", a, b, c);
    }
    {
        let a  = AtomicI32::new(100);
        let b = a.compare_exchange(150, 200, Relaxed, Relaxed);
        let c = a.load(Relaxed);

        println!("[3] a={:?}, b={:?}, c={}", a, b, c);
    }
}
