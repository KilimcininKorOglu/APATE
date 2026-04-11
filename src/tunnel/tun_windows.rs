use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl WindowsTunAdapter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            mtu: 1500,
            opened: false,
            loopback_queue: VecDeque::new(),
        }
    }
}

impl TunnelAdapter for WindowsTunAdapter {
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("wintun") && !self.name.starts_with("apate") {
            return Err(TunnelError::OpenFailed);
        }

        self.opened = true;
        Ok(())
    }

    fn configure(&mut self, mtu: u16) -> Result<(), TunnelError> {
        if !self.opened || !(576..=9000).contains(&mtu) {
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

#[cfg(test)]
mod tests {
    use crate::tunnel::TunnelAdapter;
    use crate::tunnel::packet::TunnelPacket;
    use crate::tunnel::tun_windows::WindowsTunAdapter;

    #[test]
    fn windows_adapter_accepts_valid_name() {
        let mut adapter = WindowsTunAdapter::new(String::from("wintun0"));
        adapter.open().expect("windows open");
        adapter.configure(1500).expect("windows configure");

        let packet = TunnelPacket::parse(&[
            0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        ])
        .expect("packet");
        adapter.write_packet(packet).expect("windows write");

        assert!(adapter.read_packet().expect("windows read").is_some());
    }
}
