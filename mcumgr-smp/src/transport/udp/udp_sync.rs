// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.

use crate::transport::error::Error;
use crate::transport::smp::SmpTransport;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

const BUF_SIZE: usize = 1500;

pub struct UdpTransport {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl UdpTransport {
    pub fn new<A: ToSocketAddrs>(target: A) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0))?; // Switch to Ipv4Addr
        socket.connect(target)?;

        let buf = vec![0; BUF_SIZE]; 

        Ok(Self { socket, buf })
    }

    pub fn recv_timeout(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        self.socket.set_read_timeout(timeout)?;
        Ok(())
    }
}

impl SmpTransport for UdpTransport {
    fn send(&mut self, frame: Vec<u8>) -> Result<(), Error> {
        self.socket.send(&frame)?;
        Ok(())
    }

    fn receive(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.socket.recv(&mut self.buf)?;

        Ok(Vec::from(&self.buf[0..len]))
    }
}
/// Unit tests for setting the buffer size and recieve timeout
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sets_buffer_size() {
        let transport = UdpTransport::new("192.168.2.105:1337").unwrap();
        assert_eq!(transport.buf.len(), BUF_SIZE);
    }

    #[test]
    fn test_recv_timeout_set_and_clear() {
        let mut transport = UdpTransport::new("192.168.2.105:1337").unwrap();
        assert!(transport.recv_timeout(Some(Duration::from_secs(1))).is_ok());
        assert!(transport.recv_timeout(None).is_ok());
    }
}