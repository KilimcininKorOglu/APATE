use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

pub struct FreeBsdTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    fd: Option<i32>,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl FreeBsdTunAdapter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            mtu: 1500,
            opened: false,
            fd: None,
            loopback_queue: VecDeque::new(),
        }
    }

    pub fn raw_fd(&self) -> Option<i32> {
        self.fd
    }
}

impl TunnelAdapter for FreeBsdTunAdapter {
    #[cfg(target_os = "freebsd")]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("tun") {
            return Err(TunnelError::OpenFailed);
        }

        let dev_path = format!("/dev/{}\0", self.name);
        let fd = unsafe { libc::open(dev_path.as_ptr().cast(), libc::O_RDWR | libc::O_NONBLOCK) };

        if fd < 0 {
            return Err(TunnelError::OpenFailed);
        }

        self.fd = Some(fd);
        self.opened = true;
        Ok(())
    }

    #[cfg(not(target_os = "freebsd"))]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("tun") {
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

        #[cfg(target_os = "freebsd")]
        if let Some(fd) = self.fd {
            let mut buf = [0u8; 65536];
            let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 4 {
                return Ok(None);
            }
            let packet =
                TunnelPacket::parse(&buf[4..n as usize]).map_err(|_| TunnelError::InvalidPacket)?;
            return Ok(Some(packet));
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

        #[cfg(target_os = "freebsd")]
        if let Some(fd) = self.fd {
            let data = packet.as_bytes();
            let af_header: [u8; 4] = match packet.ip_version() {
                crate::tunnel::packet::IpVersion::V4 => [0, 0, 0, 2],
                crate::tunnel::packet::IpVersion::V6 => [0, 0, 0, 28],
            };
            let mut frame = Vec::with_capacity(4 + data.len());
            frame.extend_from_slice(&af_header);
            frame.extend_from_slice(data);
            let written = unsafe { libc::write(fd, frame.as_ptr().cast(), frame.len()) };
            if written < 0 {
                return Err(TunnelError::Io);
            }
            return Ok(());
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

impl Drop for FreeBsdTunAdapter {
    fn drop(&mut self) {
        #[cfg(target_os = "freebsd")]
        if let Some(fd) = self.fd {
            unsafe {
                libc::close(fd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tunnel::TunnelAdapter;
    use crate::tunnel::packet::TunnelPacket;
    use crate::tunnel::tun_freebsd::FreeBsdTunAdapter;

    #[test]
    fn freebsd_adapter_loopback_packet() {
        let mut adapter = FreeBsdTunAdapter::new(String::from("tun3"));
        if adapter.open().is_err() {
            return;
        }
        adapter.configure(1500).expect("freebsd configure");

        let packet = TunnelPacket::parse(&[
            0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        ])
        .expect("packet");
        adapter.write_packet(packet).expect("freebsd write");

        assert!(adapter.read_packet().expect("freebsd read").is_some());
    }
}
