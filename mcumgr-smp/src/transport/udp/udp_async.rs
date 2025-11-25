// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.

use crate::transport::error::Error;
use crate::transport::smp::SmpTransportAsync;
use async_trait::async_trait;
use std::io;
use std::net::{Ipv6Addr, SocketAddr};
use tokio::net::{lookup_host, ToSocketAddrs, UdpSocket};

const BUF_SIZE: usize = 1500;

pub struct UdpTransportAsync {
    socket: UdpSocket,
    buf: Vec<u8>,
    target_addr: Option<SocketAddr>,
}

impl UdpTransportAsync {
    pub async fn new<A: ToSocketAddrs>(target: &A) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0)).await?;
        let mut addrs = lookup_host(target).await?;
        let target_addr = addrs
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no addresses"))?;
        socket.connect(target).await?;

        let buf = vec![0; BUF_SIZE];

        Ok(Self { socket, buf, target_addr:Some(target_addr) })
    }

    pub async fn new_server<A: ToSocketAddrs>(bind_addr: A) -> Result<Self, io::Error> {
        let socket: UdpSocket = UdpSocket::bind(bind_addr).await?;
        Ok(Self { socket, buf: vec![0; BUF_SIZE], target_addr: None })
    }
}

#[async_trait]
impl SmpTransportAsync for UdpTransportAsync {
    async fn send(&mut self, frame: Vec<u8>) -> Result<(), Error> {
        self.socket.send(&frame).await?;
        Ok(())
    }

    async fn send_to(&mut self, frame: Vec<u8>) -> Result<(), Error> {
        self.socket.send_to(&frame, self.target_addr.unwrap()).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>, Error> {
        let (len, addr) = self.socket.recv_from(&mut self.buf).await?;
        self.target_addr = Some(addr);
        Ok(Vec::from(&self.buf[0..len]))
    }
}
