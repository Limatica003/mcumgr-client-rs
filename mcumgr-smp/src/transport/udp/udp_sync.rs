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
    target_addr: Option<SocketAddr>,
}

impl UdpTransport {
    pub fn new<A: ToSocketAddrs>(target: A) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0))?; // Switch to Ipv4Addr
        let mut addrs = target.to_socket_addrs()?;
        let target_addr = addrs
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no addresses"))?;
        socket.connect(target)?;
       
        let buf = vec![0; BUF_SIZE]; 

        Ok(Self { socket, buf, target_addr:Some(target_addr) })
    }

    pub fn new_server<A: ToSocketAddrs>(bind_addr: A) -> Result<Self, io::Error> {
        let socket: UdpSocket = UdpSocket::bind(bind_addr)?;
        Ok(Self { socket, buf: vec![0; BUF_SIZE], target_addr: None })
    }

    pub fn recv_timeout(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        self.socket.set_read_timeout(timeout)?;
        Ok(())
    }

    pub fn send_to(){

    }
}

impl SmpTransport for UdpTransport {
    fn send(&mut self, frame: Vec<u8>) -> Result<(), Error> {
        self.socket.send(&frame)?;
        Ok(())
    }

    fn send_to(&mut self, frame: Vec<u8>) -> Result<(), Error> {
        self.socket.send_to(&frame, self.target_addr.unwrap())?;
        Ok(())
    }

    fn receive(&mut self) -> Result<Vec<u8>, Error> {
        let (len, addr) = self.socket.recv_from(&mut self.buf)?;
        self.target_addr = Some(addr);

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