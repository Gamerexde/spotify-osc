use std::time::Duration;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use tokio::net::UdpSocket;

pub fn encode_packet(address: String, data: Vec<OscType>) -> rosc::Result<Vec<u8>> {
    encoder::encode(&OscPacket::Message(OscMessage {
        addr: address,
        args: data,
    }))
}

pub async fn send_to_delay(sock: &UdpSocket, buf: &[u8], address: &String, delay: Duration) {
    sock.send_to(&buf, &address).await.unwrap();
    tokio::time::sleep(delay).await;
}