use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::SystemTime,
};
use tokio::{io::Result, net::UdpSocket, select, time::Duration};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const IFACE: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const ADDR: SocketAddrV4 = SocketAddrV4::new(IFACE, 5123);

struct Bookie {
    entries: HashMap<String, (String, u16, SystemTime)>,
}

impl Bookie {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, msg: &Discover) {
        self.entries.insert(
            msg.uuid.to_string(),
            (msg.ip.to_string(), msg.port, std::time::SystemTime::now()),
        );
    }

    pub fn purge(self) -> Self {
        let entries: HashMap<String, (String, u16, SystemTime)> = self
            .entries
            .into_iter()
            .filter(|(hash, (ip, port, time))| {
                if SystemTime::now()
                    .duration_since(time.clone())
                    .unwrap()
                    .as_secs()
                    > 60
                {
                    println!("PURGING {hash}@{ip}:{port}");
                    false
                } else {
                    true
                }
            })
            .collect();

        Self { entries }
    }
}

async fn listen(mut tx: tokio::sync::mpsc::Sender<(Ipv4Addr, u16)>) -> Result<()> {
    let socket = UdpSocket::bind(ADDR).await?;
    socket.join_multicast_v4(MULTICAST_ADDR, IFACE)?;

    let mut bookie = Bookie::new();

    loop {
        let mut buf = [0u8; 2048];
        if let Ok(Ok((len, source))) = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            socket.recv_from(&mut buf),
        )
        .await
        {
            let mut packet = bincode::deserialize::<Discover>(&buf[..len]).unwrap();
            packet.ip = match source.ip() {
                IpAddr::V4(addr) => addr,
                _ => panic!(),
            };

            println!("DISCOVERY: {packet:?}");
            bookie.insert(&packet);

            tx.try_send((packet.ip, packet.port));
        } else {
            bookie = bookie.purge();
        }
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
    uuid: String,
    ip: Ipv4Addr,
    port: u16,
}

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (tx_listener, mut rx_listener) = tokio::sync::mpsc::channel(10);

    let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let running_on = socket.local_addr().unwrap();
    let uuid = machine_uid::get().unwrap();

    tokio::spawn(async move {
        cast(
            rx,
            Discover {
                uuid,
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
        listen(tx_listener).await.unwrap();
    });

    loop {
        let mut buf = [0; 1024];

        select! {
            val = rx_listener.recv() => {
                let (ip, port) = val.unwrap();
                socket.send_to("hi".as_bytes(), format!("{}:{}", ip, port)).await.unwrap();
            }
            Ok((data, source)) = socket.recv_from(&mut buf) => {
                println!(
                    "{data} bytes of data from {}: {}",
                    source.ip(),
                    String::from_utf8_lossy(&buf[..data])
                );
            }
        }
    }
}
