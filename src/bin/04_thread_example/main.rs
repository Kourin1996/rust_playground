use std::thread;

fn main() {
    // No.1
    let t1 = thread::spawn(f);
    let t2 = thread::spawn(f);

    println!(
        "Hello, world from main thread, id: {:?}",
        thread::current().id()
    );

    t1.join().unwrap();
    t2.join().unwrap();

    // No.2
    let numbers = vec![1, 2, 3];
    let t3 = thread::spawn(move || {
        let len = numbers.len();
        let sum = numbers.into_iter().sum::<usize>();
        sum / len
    });
    let res = t3.join().unwrap();
    println!("average: {}", res);

    // No.3
    let numbers = vec![1, 2, 3];
    thread::scope(|s| {
        println!("entering scope, id={:?}", thread::current().id());

        s.spawn(|| {
            println!("entering thread 3-1, id={:?}", thread::current().id());

            println!("length: {}", numbers.len());
        });

        s.spawn(|| {
            println!("entering thread 3-2, id={:?}", thread::current().id());

            for n in &numbers {
                println!("number: {}", n);
            }
        });
    });
    println!("main thread, numbers: {:?}", numbers);
}

fn f() {
    let id = thread::current().id();
    println!("Hello world from another thread! id: {:?}", id);
}
