use std::error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use async_std::future::timeout;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task::spawn;

use futures::join;

async fn handle(mut sock: TcpStream, addr: SocketAddr) -> Result<bool, Box<dyn error::Error>> {
    let mut ret = true;
    let mut buf = [0; 32];
    sock.read(&mut buf).await?;
    let s = String::from_utf8(buf.to_vec()).unwrap()
        .replace('\u{0}', "")
        .replace("\\n", "")
        .trim_end()
        .to_string();
    println!("[{:?}] Read from {:?}:\n\t{:?}", thread::current().id(), addr, s);

    match s.as_ref() {
        "die" => {
            sock.write_all(b"Stopping\n").await?;
            ret = false;
        },
        _ => sock.write_all(b"OK\n").await?,
    }
    sock.shutdown(std::net::Shutdown::Both)?;
    Ok(ret)
}

async fn listen<'a>(addr: &str) -> Result<(), Box<dyn error::Error>> {
    let flag = Arc::new(AtomicBool::new(true));

    println!("[{:?}] Binding on {:?}", thread::current().id(), addr);
    let listener = TcpListener::bind(addr).await?;

    while flag.load(Ordering::Acquire) {
        match timeout(Duration::from_millis(100), listener.accept()).await {
            Ok(r) => match r {
                Ok((stream, remote_addr)) => {
                    println!("[{:?}] Got connection on {:?}", thread::current().id(), remote_addr);
                    let cloned_flag = Arc::clone(&flag);
                    spawn(async move {
                        let ret = handle(stream, remote_addr).await.unwrap();
                        cloned_flag.swap(ret, Ordering::Acquire);
                    });
                },
                Err(_) => (),
            },
            Err(_) => (),
        }
    }
    println!("Done listening on {:?}", addr);
    Ok(())
}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    println!("This is the way.");
    println!("My pid is: {:?}, tid: {:?}", std::process::id(), thread::current().id());


    let f_1 = spawn(async { listen("127.0.0.1:9990").await.unwrap() });
    let f_2 = spawn(async { listen("127.0.0.1:9991").await.unwrap() });
    let f_3 = spawn(async { listen("127.0.0.1:9992").await.unwrap() });

    join!(f_1, f_2, f_3);

    println!("I have spoken.");
    Ok(())
}
