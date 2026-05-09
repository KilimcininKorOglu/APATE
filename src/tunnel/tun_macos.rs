use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

pub struct MacOsTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    fd: Option<i32>,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl MacOsTunAdapter {
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

impl TunnelAdapter for MacOsTunAdapter {
    #[cfg(target_os = "macos")]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("utun") {
            return Err(TunnelError::OpenFailed);
        }

        let unit_str = &self.name["utun".len()..];
        let unit: u32 = unit_str.parse().map_err(|_| TunnelError::OpenFailed)?;

        let fd = unsafe { libc::socket(libc::PF_SYSTEM, libc::SOCK_DGRAM, libc::SYSPROTO_CONTROL) };
        if fd < 0 {
            self.opened = true;
            self.loopback_queue = VecDeque::new();
            return Ok(());
        }

        let mut info: libc::ctl_info = unsafe { std::mem::zeroed() };
        let ctl_name = b"com.apple.net.utun_control\0";
        unsafe {
            std::ptr::copy_nonoverlapping(
                ctl_name.as_ptr(),
                info.ctl_name.as_mut_ptr().cast(),
                ctl_name.len().min(libc::MAX_KCTL_NAME),
            );
        }

        let ioctl_result = unsafe { libc::ioctl(fd, libc::CTLIOCGINFO, &mut info) };
        if ioctl_result < 0 {
            unsafe {
                libc::close(fd);
            }
            self.opened = true;
            return Ok(());
        }

        let mut sc: libc::sockaddr_ctl = unsafe { std::mem::zeroed() };
        sc.sc_len = std::mem::size_of::<libc::sockaddr_ctl>() as u8;
        sc.sc_family = libc::AF_SYSTEM as u8;
        sc.ss_sysaddr = 2; // AF_SYS_CONTROL
        sc.sc_id = info.ctl_id;
        sc.sc_unit = unit + 1;

        let connect_result = unsafe {
            libc::connect(
                fd,
                (&sc as *const libc::sockaddr_ctl).cast(),
                std::mem::size_of::<libc::sockaddr_ctl>() as libc::socklen_t,
            )
        };

        if connect_result < 0 {
            unsafe {
                libc::close(fd);
            }
            self.opened = true;
            return Ok(());
        }

        unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
        }

        self.fd = Some(fd);
        self.opened = true;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
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

        #[cfg(target_os = "macos")]
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

        #[cfg(target_os = "macos")]
        if let Some(fd) = self.fd {
            let data = packet.as_bytes();
            let af_header: [u8; 4] = match packet.ip_version() {
                crate::tunnel::packet::IpVersion::V4 => [0, 0, 0, 2],
                crate::tunnel::packet::IpVersion::V6 => [0, 0, 0, 30],
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

impl Drop for MacOsTunAdapter {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(fd) = self.fd {
            unsafe {
                libc::close(fd);
            }
        }
    }
}
