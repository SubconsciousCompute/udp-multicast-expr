use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::{io::Result, net::UdpSocket, time::Duration};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const IFACE: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const ADDR: SocketAddrV4 = SocketAddrV4::new(IFACE, 5123);

async fn listen() -> Result<()> {
    let socket = UdpSocket::bind(ADDR).await?;
    socket.join_multicast_v4(MULTICAST_ADDR, IFACE)?;

    loop {
        let mut buf = [0u8; 1024];
        let (len, source) = socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&mut buf);
        println!("{source}: {msg}")
    }

    Ok(())
}

async fn cast(mut rx: tokio::sync::mpsc::Receiver<String>) -> Result<()> {
    let socket = UdpSocket::bind(SocketAddrV4::new(IFACE, 0)).await?;
    socket
        .connect(SocketAddrV4::new(MULTICAST_ADDR, 5123))
        .await?;
    socket.set_multicast_loop_v4(false)?;

    loop {
        if let Ok(msg) = rx.try_recv() {
            socket.send(msg.as_bytes()).await?;
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Message {}

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    tokio::spawn(async {
        cast(rx).await.unwrap();
    });
    tokio::spawn(async {
        listen().await.unwrap();
    });

    loop {
        let mut msg = String::new();
        std::io::stdin().read_line(&mut msg).unwrap();
        tx.send(msg).await.unwrap();
    }
}
