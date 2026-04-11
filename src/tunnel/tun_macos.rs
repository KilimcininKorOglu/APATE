use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacOsTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl MacOsTunAdapter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            mtu: 1500,
            opened: false,
            loopback_queue: VecDeque::new(),
        }
    }
}

impl TunnelAdapter for MacOsTunAdapter {
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("utun") {
            return Err(TunnelError::OpenFailed);
        }

        self.opened = true;
        Ok(())
    }

    fn configure(&mut self, mtu: u16) -> Result<(), TunnelError> {
        if !self.opened || !(1280..=9000).contains(&mtu) {
            return Err(TunnelError::ConfigureFailed);
        }

        self.mtu = mtu;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Option<TunnelPacket>, TunnelError> {
        if !self.opened {
            return Err(TunnelError::Io);
        }

        Ok(self.loopback_queue.pop_front())
    }

    fn write_packet(&mut self, packet: TunnelPacket) -> Result<(), TunnelError> {
        if !self.opened {
            return Err(TunnelError::Io);
        }
        if packet.as_bytes().len() > usize::from(self.mtu) {
            return Err(TunnelError::InvalidPacket);
        }

        self.loopback_queue.push_back(packet);
        Ok(())
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn name(&self) -> &str {
        &self.name
    }
}
