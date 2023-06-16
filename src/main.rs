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
        let mut buf = [0u8; 2048];
        let (len, source) = socket.recv_from(&mut buf).await?;
        let mut packet = bincode::deserialize::<Discover>(&buf[..len]).unwrap();
        packet.ip = match source.ip() {
            IpAddr::V4(addr) => addr,
            _ => panic!(),
        };

        println!("DISCOVERY: {packet:?}")
    }

    Ok(())
}

async fn cast(mut rx: tokio::sync::mpsc::Receiver<String>, msg: Discover) -> Result<()> {
    let socket = UdpSocket::bind(SocketAddrV4::new(IFACE, 0)).await?;
    socket
        .connect(SocketAddrV4::new(MULTICAST_ADDR, 5123))
        .await?;
    socket.set_multicast_loop_v4(false)?;

    loop {
        socket
            .send(&bincode::serialize(&msg).unwrap())
            .await
            .unwrap();
        std::thread::sleep(Duration::from_secs(30));
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct Discover {
    ip: Ipv4Addr,
    port: u16,
}

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let running_on = socket.local_addr().unwrap();

    tokio::spawn(async move {
        cast(
            rx,
            Discover {
                ip: match running_on.ip() {
                    IpAddr::V4(addr) => addr,
                    _ => panic!(),
                },
                port: running_on.port(),
            },
        )
        .await
        .unwrap();
    });
    tokio::spawn(async {
        listen().await.unwrap();
    });

    loop {
        let mut buf = [0; 1024];
        let (data, source) = socket.recv_from(&mut buf).await.unwrap();
        println!("{data} bytes of data from {}", source.ip());
        socket.send_to(&buf[..data], source);
    }
}
