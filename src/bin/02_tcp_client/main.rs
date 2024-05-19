use std::io::Error;
use std::process;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::sync::watch::Receiver;
use tokio::{io, signal};

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("02 TCP Client");

    let addr = "localhost:8000";
    let stream = TcpStream::connect(addr).await?;

    println!("Connected to: {}", addr);

    let (notify_shutdown_tx, notify_shutdown_rx) = watch::channel(());

    let task1 = tokio::spawn(async move {
        signal::ctrl_c().await.expect("failed to listen for ctrl-c");
        println!("ctrl-c received");
        notify_shutdown_tx
            .send(())
            .expect("failed to send shutdown notification");

        drop(notify_shutdown_tx);
    });

    let task2 = tokio::spawn(async {
        run(stream, notify_shutdown_rx)
            .await
            .expect("failed to run client");
    });

    let _ = tokio::join!(task1, task2);

    println!("graceful shutdown completed, bye");

    process::exit(0);

    Ok(())
}

async fn run(stream: TcpStream, shutdown_notification: Receiver<()>) -> Result<(), Error> {
    let (read, mut write) = tokio::io::split(stream);

    let mut shutdown_subscription = shutdown_notification.clone();
    let task1 = tokio::spawn(async move {
        let mut reader = BufReader::new(read);
        let mut buf = vec![0; 1024];

        println!("receiving process started");

        loop {
            let n = tokio::select! {
                res = reader.read(&mut buf) => {
                    res.expect("failed to read data from stream")
                }
                _ = shutdown_subscription.changed() => {
                    println!("shutdown received, closing receiving process");
                    return ();
                }
            };

            if n == 0 {
                // TODO: should close all tasks when a peer closes the connection safely
                println!("read zero size, bye");
                return ();
            }

            let msg =
                String::from_utf8(buf[0..n].to_vec()).expect("failed to convert [u8] to String");

            println!("received: {}", msg);
        }
    });

    let mut shutdown_subscription = shutdown_notification.clone();
    let task2 = tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();

        println!("sending process started");

        loop {
            let next_line = tokio::select! {
                res = reader.next_line() => {
                    let line = res.expect("failed to read line from stdin");

                    match line {
                        Some(line) => line,
                        None => {
                            println!("stdin closed, bye");
                            return ();
                        }
                    }
                },
                _ = shutdown_subscription.changed() => {
                    println!("shutdown received, closing sending process");
                    return ();
                }
            };

            write
                .write_all(next_line.as_bytes())
                .await
                .expect("failed to write data to stream");

            println!("sent: {}", next_line);
        }
    });

    // awaits all tasks in join_set to complete
    let _ = tokio::join!(task1, task2);

    println!("all tasks completed");

    Ok(())
}
