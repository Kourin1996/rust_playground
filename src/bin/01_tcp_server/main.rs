mod echo_application;

use crate::echo_application::{EchoApplication, EchoApplicationError};
use std::future::Future;
use std::io::Error;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("01 TCP Server");

    let addr = "localhost:8000";
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    run(listener, tokio::signal::ctrl_c()).await?;

    Ok(())
}

async fn run(
    tcp_listener: TcpListener,
    shutdown: impl Future + Send + 'static,
) -> Result<(), Error> {
    let mut num_connections = 0;

    let (tx, rx) = watch::channel(());
    tokio::spawn(async move {
        shutdown.await;
        println!("shutdown signal sent");
        tx.send(()).unwrap();
    });

    loop {
        let mut rx = rx.clone();
        let socket = select! {
            _ = rx.changed() => {
                println!("shutdown signal received");
                break;
            }
            res = tcp_listener.accept() => {
                match res {
                    Ok((socket, _)) => {
                        socket
                    },
                    Err(e) => {
                        eprintln!("failed to accept socket; error = {:?}", e);
                        continue;
                    }
                }
            }
        };

        let (read, write) = tokio::io::split(socket);

        num_connections += 1;
        let index = num_connections;

        println!("new connection: {}", index);

        let mut application = EchoApplication::new(read, write, index);

        let mut rx = rx.clone();
        tokio::spawn(async move {
            select! {
                _ = rx.changed() => {
                    println!("con {}: shutdown signal received", index);
                    return;
                },
                res = application.run() => {
                    match res {
                        Ok(_) => {
                            println!("con {}: connection closed", index);
                        },
                        Err(e) => {
                            match e {
                                EchoApplicationError::IOError(e) => {
                                    eprintln!("con {}: IO error: {}", index, e);
                                },
                                EchoApplicationError::DecodeError(e) => {
                                    eprintln!("con {}: decode error: {}", index, e);
                                },
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}
