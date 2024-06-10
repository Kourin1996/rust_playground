use std::cell::{Cell, RefCell};

fn main() {
    println!("Hello, world!");

    let a = Cell::new(1);
    let b = Cell::new(1);
    f1(&a, &b);

    let a = Cell::new(1);
    let b = &a;
    f1(&a, &b);

    let a = RefCell::new(1);
    f2(&a);
    println!("RefCell value: {}", a.borrow());
}

fn f1(a: &Cell<i32>, b: &Cell<i32>) {
    let before = a.get();
    println!("before: {}", before);

    b.set(b.get() + 1);

    let after = a.get();
    println!("after: {}", after);

    if before != after {
        println!("a was changed");
    }
}

fn f2(a: &RefCell<i32>) {
    let mut a = a.borrow_mut();
    *a += 1;
}
