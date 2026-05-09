use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

#[cfg(target_os = "linux")]
const TUNSETIFF: libc::c_ulong = 0x400454CA;
#[cfg(target_os = "linux")]
const IFF_TUN: libc::c_short = 0x0001;
#[cfg(target_os = "linux")]
const IFF_NO_PI: libc::c_short = 0x1000;

pub struct LinuxTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    fd: Option<i32>,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl LinuxTunAdapter {
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

impl TunnelAdapter for LinuxTunAdapter {
    #[cfg(target_os = "linux")]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("tun") && !self.name.starts_with("apate") {
            return Err(TunnelError::OpenFailed);
        }

        let fd = unsafe {
            libc::open(
                b"/dev/net/tun\0".as_ptr().cast(),
                libc::O_RDWR | libc::O_NONBLOCK,
            )
        };

        if fd < 0 {
            self.opened = true;
            return Ok(());
        }

        #[repr(C)]
        struct Ifreq {
            ifr_name: [u8; 16],
            ifr_flags: libc::c_short,
            _pad: [u8; 22],
        }

        let mut ifr: Ifreq = unsafe { std::mem::zeroed() };
        let name_bytes = self.name.as_bytes();
        let copy_len = name_bytes.len().min(15);
        ifr.ifr_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        ifr.ifr_flags = IFF_TUN | IFF_NO_PI;

        let ioctl_result = unsafe { libc::ioctl(fd, TUNSETIFF, &mut ifr) };

        if ioctl_result < 0 {
            unsafe { libc::close(fd); }
            self.opened = true;
            return Ok(());
        }

        self.fd = Some(fd);
        self.opened = true;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("tun") && !self.name.starts_with("apate") {
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

        #[cfg(target_os = "linux")]
        if let Some(fd) = self.fd {
            let mut buf = [0u8; 65536];
            let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                return Ok(None);
            }
            let packet = TunnelPacket::parse(&buf[..n as usize])
                .map_err(|_| TunnelError::InvalidPacket)?;
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

        #[cfg(target_os = "linux")]
        if let Some(fd) = self.fd {
            let data = packet.as_bytes();
            let written = unsafe { libc::write(fd, data.as_ptr().cast(), data.len()) };
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

impl Drop for LinuxTunAdapter {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(fd) = self.fd {
            unsafe { libc::close(fd); }
        }
    }
}
