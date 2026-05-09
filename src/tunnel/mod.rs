pub mod packet;
#[cfg(any(target_os = "freebsd", test))]
pub mod tun_freebsd;
pub mod tun_linux;
pub mod tun_macos;
#[cfg(any(target_os = "windows", test))]
pub mod tun_windows;

pub use packet::{IpVersion, TunnelPacket};
#[cfg(any(target_os = "freebsd", test))]
pub use tun_freebsd::FreeBsdTunAdapter;
pub use tun_linux::LinuxTunAdapter;
pub use tun_macos::MacOsTunAdapter;
#[cfg(any(target_os = "windows", test))]
pub use tun_windows::WindowsTunAdapter;

use thiserror::Error;

pub trait TunnelAdapter {
    fn open(&mut self) -> Result<(), TunnelError>;
    fn configure(&mut self, mtu: u16) -> Result<(), TunnelError>;
    fn read_packet(&mut self) -> Result<Option<TunnelPacket>, TunnelError>;
    fn write_packet(&mut self, packet: TunnelPacket) -> Result<(), TunnelError>;
    fn mtu(&self) -> u16;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TunnelError {
    #[error("tunnel open failed")]
    OpenFailed,
    #[error("tunnel configuration failed")]
    ConfigureFailed,
    #[error("tunnel io failure")]
    Io,
    #[error("invalid packet")]
    InvalidPacket,
}

#[cfg(test)]
mod tests {
    use crate::tunnel::packet::TunnelPacket;
    use crate::tunnel::tun_linux::LinuxTunAdapter;
    use crate::tunnel::tun_macos::MacOsTunAdapter;
    use crate::tunnel::{TunnelAdapter, TunnelError};

    fn ipv4_packet() -> TunnelPacket {
        TunnelPacket::parse(&[
            0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        ])
        .expect("valid ipv4 packet")
    }

    #[test]
    fn linux_adapter_loopback_exchange() {
        let mut adapter = LinuxTunAdapter::new(String::from("tun0"));
        if adapter.open().is_err() {
            return;
        }
        adapter.configure(1500).expect("configure linux");

        adapter.write_packet(ipv4_packet()).expect("write packet");
        let received = adapter
            .read_packet()
            .expect("read result")
            .expect("packet expected");

        assert_eq!(20, received.as_bytes().len());
    }

    #[test]
    fn macos_adapter_rejects_invalid_mtu() {
        let mut adapter = MacOsTunAdapter::new(String::from("utun2"));
        if adapter.open().is_err() {
            return;
        }
        let error = adapter.configure(100).expect_err("invalid mtu must fail");

        assert_eq!(TunnelError::ConfigureFailed, error);
    }
}
