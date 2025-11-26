// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.

use crate::transport::error::Error;
use crate::transport::smp::SmpTransportAsync;
use async_trait::async_trait;
use std::io;
use std::net::{Ipv6Addr, SocketAddr};
use tokio::net::{lookup_host, ToSocketAddrs, UdpSocket};
use tokio::time::{timeout, Duration};

const BUF_SIZE: usize = 1500;

pub struct UdpTransportAsync {
    socket: UdpSocket,
    buf: Vec<u8>,
    target_addr: Option<SocketAddr>,
    pub local_addr: SocketAddr,
    timeout: Option<Duration>,
}

impl UdpTransportAsync {
    pub async fn new<A: ToSocketAddrs>(target: &A, timeout: Option<Duration>) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0)).await?;
        let mut addrs = lookup_host(target).await?;
        let target_addr = addrs
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no addresses"))?;
        socket.connect(target).await?;
        let local_addr = socket.local_addr().unwrap();
        let buf = vec![0; BUF_SIZE];

        Ok(Self { socket, buf, target_addr:Some(target_addr), local_addr, timeout})
    }

    pub async fn new_server<A: ToSocketAddrs>(bind_addr: A) -> Result<Self, io::Error> {
        let socket: UdpSocket = UdpSocket::bind(bind_addr).await?;
        let local_addr = socket.local_addr().unwrap();
        Ok(Self { socket, buf: vec![0; BUF_SIZE], target_addr: None, local_addr, timeout: None})
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
        // If no timeout is configured -> plain recv_from
        let Some(dur) = self.timeout else {
            let (len, addr) = self.socket.recv_from(&mut self.buf).await?;
            self.target_addr = Some(addr);
            return Ok(self.buf[..len].to_vec());
        };

        // With timeout
        let recv_result = timeout(dur, self.socket.recv_from(&mut self.buf)).await;

        match recv_result {
            // recv_from finished before timeout
            Ok(Ok((len, addr))) => {
                self.target_addr = Some(addr);
                Ok(self.buf[..len].to_vec())
            }
            // recv_from returned an io::Error
            Ok(Err(e)) => Err(e.into()),
            // timeout fired before any packet arrived
            Err(elapsed) => {
                let io_err = io::Error::new(io::ErrorKind::TimedOut, elapsed);
                Err(io_err.into())
            }
        }
    }
}
