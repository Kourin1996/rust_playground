use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Error, Debug)]
pub enum EchoApplicationError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error("UTF-8 format error")]
    DecodeError(#[from] std::string::FromUtf8Error),
}

#[derive(Debug)]
pub struct EchoApplication<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: R,
    writer: W,
    index: u8, // for debug
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> EchoApplication<R, W> {
    pub fn new(reader: R, writer: W, index: u8) -> Self {
        Self {
            reader,
            writer,
            index,
        }
    }

    pub async fn run(&mut self) -> Result<(), EchoApplicationError> {
        let mut buf = vec![0; 1024];

        loop {
            let n = self.reader.read(&mut buf).await?;
            // throws error when a peer closes the connection suddenly

            if n == 0 {
                // XXX: reach here when a peer closes the connection safely
                println!("con {}: read zero size, bye", self.index);
                return Ok(());
            }

            let msg = String::from_utf8(buf[0..n].to_vec())?;

            println!("con {}: received {}", self.index, msg);

            match msg.as_str() {
                "close" => {
                    println!("con {}: received close, bye", self.index);

                    return Ok(());
                }
                _ => self.writer.write_all(&buf[0..n]).await?,
            };
        }
    }
}
