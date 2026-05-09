use crate::tunnel::{TunnelAdapter, TunnelError, TunnelPacket};
use std::collections::VecDeque;

#[cfg(target_os = "windows")]
mod wintun {
    use std::ffi::c_void;

    pub type WintunAdapterHandle = *mut c_void;
    pub type WintunSessionHandle = *mut c_void;

    pub type WintunCreateAdapterFn =
        unsafe extern "stdcall" fn(*const u16, *const u16, *const c_void) -> WintunAdapterHandle;
    pub type WintunCloseAdapterFn = unsafe extern "stdcall" fn(WintunAdapterHandle);
    pub type WintunStartSessionFn =
        unsafe extern "stdcall" fn(WintunAdapterHandle, u32) -> WintunSessionHandle;
    pub type WintunEndSessionFn = unsafe extern "stdcall" fn(WintunSessionHandle);
    pub type WintunReceivePacketFn =
        unsafe extern "stdcall" fn(WintunSessionHandle, *mut u32) -> *mut u8;
    pub type WintunReleaseReceivePacketFn =
        unsafe extern "stdcall" fn(WintunSessionHandle, *const u8);
    pub type WintunAllocateSendPacketFn =
        unsafe extern "stdcall" fn(WintunSessionHandle, u32) -> *mut u8;
    pub type WintunSendPacketFn = unsafe extern "stdcall" fn(WintunSessionHandle, *const u8);

    pub struct WintunApi {
        pub _lib: windows_sys::Win32::Foundation::HMODULE,
        pub create_adapter: WintunCreateAdapterFn,
        pub close_adapter: WintunCloseAdapterFn,
        pub start_session: WintunStartSessionFn,
        pub end_session: WintunEndSessionFn,
        pub receive_packet: WintunReceivePacketFn,
        pub release_receive_packet: WintunReleaseReceivePacketFn,
        pub allocate_send_packet: WintunAllocateSendPacketFn,
        pub send_packet: WintunSendPacketFn,
    }

    impl WintunApi {
        pub unsafe fn load() -> Option<Self> {
            use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

            let lib = LoadLibraryA(c"wintun.dll".as_ptr() as *const u8);
            if lib.is_null() {
                return None;
            }

            macro_rules! load_fn {
                ($name:literal) => {
                    std::mem::transmute(GetProcAddress(lib, concat!($name, "\0").as_ptr())?)
                };
            }

            Some(Self {
                _lib: lib,
                create_adapter: load_fn!("WintunCreateAdapter"),
                close_adapter: load_fn!("WintunCloseAdapter"),
                start_session: load_fn!("WintunStartSession"),
                end_session: load_fn!("WintunEndSession"),
                receive_packet: load_fn!("WintunReceivePacket"),
                release_receive_packet: load_fn!("WintunReleaseReceivePacket"),
                allocate_send_packet: load_fn!("WintunAllocateSendPacket"),
                send_packet: load_fn!("WintunSendPacket"),
            })
        }
    }

    pub fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

pub struct WindowsTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    #[cfg(target_os = "windows")]
    api: Option<wintun::WintunApi>,
    #[cfg(target_os = "windows")]
    adapter: wintun::WintunAdapterHandle,
    #[cfg(target_os = "windows")]
    session: wintun::WintunSessionHandle,
    loopback_queue: VecDeque<TunnelPacket>,
}

impl WindowsTunAdapter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            mtu: 1500,
            opened: false,
            #[cfg(target_os = "windows")]
            api: None,
            #[cfg(target_os = "windows")]
            adapter: std::ptr::null_mut(),
            #[cfg(target_os = "windows")]
            session: std::ptr::null_mut(),
            loopback_queue: VecDeque::new(),
        }
    }

    pub fn raw_fd(&self) -> Option<i32> {
        None
    }
}

impl TunnelAdapter for WindowsTunAdapter {
    #[cfg(target_os = "windows")]
    fn open(&mut self) -> Result<(), TunnelError> {
        if !self.name.starts_with("wintun") && !self.name.starts_with("apate") {
            return Err(TunnelError::OpenFailed);
        }

        let api = match unsafe { wintun::WintunApi::load() } {
            Some(a) => a,
            None => return Err(TunnelError::OpenFailed),
        };

        let name_wide = wintun::to_wide(&self.name);
        let tunnel_type = wintun::to_wide("Apate");

        let adapter = unsafe {
            (api.create_adapter)(name_wide.as_ptr(), tunnel_type.as_ptr(), std::ptr::null())
        };
        if adapter.is_null() {
            return Err(TunnelError::OpenFailed);
        }

        let capacity = 0x40_0000_u32;
        let session = unsafe { (api.start_session)(adapter, capacity) };
        if session.is_null() {
            unsafe {
                (api.close_adapter)(adapter);
            }
            return Err(TunnelError::OpenFailed);
        }

        self.adapter = adapter;
        self.session = session;
        self.api = Some(api);
        self.opened = true;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
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

    #[cfg(target_os = "windows")]
    fn read_packet(&mut self) -> Result<Option<TunnelPacket>, TunnelError> {
        if !self.opened {
            return Err(TunnelError::Io);
        }

        if let Some(ref api) = self.api {
            let mut packet_size: u32 = 0;
            let ptr = unsafe { (api.receive_packet)(self.session, &mut packet_size) };
            if ptr.is_null() {
                return Ok(None);
            }

            let data = unsafe { std::slice::from_raw_parts(ptr, packet_size as usize) };
            let packet = TunnelPacket::parse(data).map_err(|_| TunnelError::InvalidPacket)?;

            unsafe {
                (api.release_receive_packet)(self.session, ptr);
            }

            return Ok(Some(packet));
        }

        Ok(self.loopback_queue.pop_front())
    }

    #[cfg(not(target_os = "windows"))]
    fn read_packet(&mut self) -> Result<Option<TunnelPacket>, TunnelError> {
        if !self.opened {
            return Err(TunnelError::Io);
        }
        Ok(self.loopback_queue.pop_front())
    }

    #[cfg(target_os = "windows")]
    fn write_packet(&mut self, packet: TunnelPacket) -> Result<(), TunnelError> {
        if !self.opened {
            return Err(TunnelError::Io);
        }
        let data = packet.as_bytes();
        if data.len() > usize::from(self.mtu) {
            return Err(TunnelError::InvalidPacket);
        }

        if let Some(ref api) = self.api {
            let buf = unsafe { (api.allocate_send_packet)(self.session, data.len() as u32) };
            if buf.is_null() {
                return Err(TunnelError::Io);
            }
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
                (api.send_packet)(self.session, buf);
            }
            return Ok(());
        }

        self.loopback_queue.push_back(packet);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
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

#[cfg(target_os = "windows")]
impl Drop for WindowsTunAdapter {
    fn drop(&mut self) {
        if let Some(ref api) = self.api {
            if !self.session.is_null() {
                unsafe {
                    (api.end_session)(self.session);
                }
            }
            if !self.adapter.is_null() {
                unsafe {
                    (api.close_adapter)(self.adapter);
                }
            }
        }
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
        if adapter.open().is_err() {
            return;
        }
        adapter.configure(1500).expect("windows configure");

        let packet = TunnelPacket::parse(&[
            0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        ])
        .expect("packet");
        adapter.write_packet(packet).expect("windows write");

        assert!(adapter.read_packet().expect("windows read").is_some());
    }
}
