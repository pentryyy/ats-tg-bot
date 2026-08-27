use crate::dto::request::frame::FrameData;
use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;

#[async_trait]
pub trait SocketServiceTrait: Send + Sync {
    async fn recv_frame(&self, buf: &mut [u8]) -> Result<(FrameData, SocketAddr)>;
}
