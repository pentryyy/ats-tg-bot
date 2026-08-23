use crate::dto::request::frame::FrameData;
use crate::traits::socket_service::SocketServiceTrait;
use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

pub struct SocketService {
    socket: UdpSocket,
}

impl SocketService {
    pub async fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(SocketService {
            socket: UdpSocket::bind(addr)?,
        })
    }

    pub async fn recv_from<T: DeserializeOwned>(&self, buf: &mut [u8]) -> Result<(T, SocketAddr)> {
        let (len, addr) = self.socket.recv_from(buf)?;
        let data = bincode::deserialize(&buf[..len])?;
        Ok((data, addr))
    }
}

#[async_trait]
impl SocketServiceTrait for SocketService {
    async fn recv_frame(&self, buf: &mut [u8]) -> Result<(FrameData, SocketAddr)> {
        self.recv_from::<FrameData>(buf).await
    }
}
