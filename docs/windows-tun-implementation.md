# Windows TUN Implementation Plan

## Status

`src/tunnel/tun_windows.rs` is currently a loopback stub. All other platforms (Linux, macOS, FreeBSD) have real kernel TUN device implementations. This document specifies exactly what needs to be done to complete the Windows TUN adapter.

## Why WinTUN

Windows has no built-in TUN/TAP kernel interface like Linux (`/dev/net/tun`) or macOS (`utun`). The standard approach is WinTUN, a lightweight TUN driver created by the WireGuard project. It provides a userspace API via `wintun.dll` that creates virtual network adapters and exchanges IP packets through ring buffers.

## Prerequisites

| Requirement                | Detail                                                      |
|----------------------------|-------------------------------------------------------------|
| Build environment          | Windows 10/11 with MSVC toolchain                           |
| WinTUN driver              | `wintun.dll` (x64) placed in the binary's directory or PATH |
| Download                   | https://www.wintun.net/ (MIT licensed)                      |
| Cargo.toml feature         | `Win32_System_LibraryLoader` added to `windows-sys` features |
| Privileges                 | Administrator required for adapter creation                 |

## Cargo.toml Change

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61.2", features = [
    "Win32_Networking_WinSock",
    "Win32_System_IO",
    "Win32_System_LibraryLoader",
    "Win32_Foundation",
] }
```

Add `Win32_System_LibraryLoader` for `LoadLibraryA` and `GetProcAddress`.

## WinTUN API Functions to Load

All functions are loaded at runtime via `LoadLibraryA("wintun.dll")` + `GetProcAddress`. Define function pointer types matching the WinTUN C API:

| Function                     | Signature                                                                                     | Purpose                          |
|------------------------------|-----------------------------------------------------------------------------------------------|----------------------------------|
| `WintunCreateAdapter`        | `(name: PCWSTR, tunnel_type: PCWSTR, guid: *const GUID) -> WINTUN_ADAPTER_HANDLE`            | Create virtual network adapter   |
| `WintunCloseAdapter`         | `(adapter: WINTUN_ADAPTER_HANDLE)`                                                           | Destroy adapter                  |
| `WintunStartSession`         | `(adapter: WINTUN_ADAPTER_HANDLE, capacity: u32) -> WINTUN_SESSION_HANDLE`                   | Start packet session (ring buf)  |
| `WintunEndSession`           | `(session: WINTUN_SESSION_HANDLE)`                                                           | End session                      |
| `WintunGetReadWaitEvent`     | `(session: WINTUN_SESSION_HANDLE) -> HANDLE`                                                 | Event handle for read readiness  |
| `WintunReceivePacket`        | `(session: WINTUN_SESSION_HANDLE, size: *mut u32) -> *mut u8`                                | Receive IP packet                |
| `WintunReleaseReceivePacket` | `(session: WINTUN_SESSION_HANDLE, packet: *const u8)`                                        | Release received packet buffer   |
| `WintunAllocateSendPacket`   | `(session: WINTUN_SESSION_HANDLE, size: u32) -> *mut u8`                                     | Allocate send buffer             |
| `WintunSendPacket`           | `(session: WINTUN_SESSION_HANDLE, packet: *const u8)`                                        | Send IP packet                   |

## Type Definitions

```rust
type WINTUN_ADAPTER_HANDLE = *mut std::ffi::c_void;
type WINTUN_SESSION_HANDLE = *mut std::ffi::c_void;

struct WintunApi {
    create_adapter:         unsafe extern "system" fn(*const u16, *const u16, *const GUID) -> WINTUN_ADAPTER_HANDLE,
    close_adapter:          unsafe extern "system" fn(WINTUN_ADAPTER_HANDLE),
    start_session:          unsafe extern "system" fn(WINTUN_ADAPTER_HANDLE, u32) -> WINTUN_SESSION_HANDLE,
    end_session:            unsafe extern "system" fn(WINTUN_SESSION_HANDLE),
    get_read_wait_event:    unsafe extern "system" fn(WINTUN_SESSION_HANDLE) -> isize,
    receive_packet:         unsafe extern "system" fn(WINTUN_SESSION_HANDLE, *mut u32) -> *mut u8,
    release_receive_packet: unsafe extern "system" fn(WINTUN_SESSION_HANDLE, *const u8),
    allocate_send_packet:   unsafe extern "system" fn(WINTUN_SESSION_HANDLE, u32) -> *mut u8,
    send_packet:            unsafe extern "system" fn(WINTUN_SESSION_HANDLE, *const u8),
}
```

## Implementation Steps

### Step 1: DLL Loader

File: `src/tunnel/tun_windows.rs`

```rust
#[cfg(target_os = "windows")]
fn load_wintun_api() -> Result<WintunApi, TunnelError> {
    use windows_sys::Win32::System::LibraryLoader::{LoadLibraryA, GetProcAddress};

    let dll = unsafe { LoadLibraryA(b"wintun.dll\0".as_ptr()) };
    if dll == 0 {
        return Err(TunnelError::OpenFailed);
    }

    // For each function:
    let create_adapter = unsafe {
        let proc = GetProcAddress(dll, b"WintunCreateAdapter\0".as_ptr());
        std::mem::transmute(proc.ok_or(TunnelError::OpenFailed)?)
    };
    // ... repeat for all 8 functions
}
```

### Step 2: Adapter Creation in `open()`

```rust
fn open(&mut self) -> Result<(), TunnelError> {
    let api = load_wintun_api()?;

    // Convert adapter name to UTF-16
    let name_wide: Vec<u16> = self.name.encode_utf16().chain(std::iter::once(0)).collect();
    let type_wide: Vec<u16> = "Apate".encode_utf16().chain(std::iter::once(0)).collect();

    let adapter = unsafe {
        (api.create_adapter)(name_wide.as_ptr(), type_wide.as_ptr(), std::ptr::null())
    };
    if adapter.is_null() {
        return Err(TunnelError::OpenFailed);
    }

    let session = unsafe { (api.start_session)(adapter, 0x400000) }; // 4MB ring
    if session.is_null() {
        unsafe { (api.close_adapter)(adapter); }
        return Err(TunnelError::OpenFailed);
    }

    self.adapter = Some(adapter);
    self.session = Some(session);
    self.api = Some(api);
    self.opened = true;
    Ok(())
}
```

### Step 3: Packet Read

```rust
fn read_packet(&mut self) -> Result<Option<TunnelPacket>, TunnelError> {
    let (api, session) = self.api_and_session()?;

    let mut size: u32 = 0;
    let ptr = unsafe { (api.receive_packet)(session, &mut size) };
    if ptr.is_null() {
        return Ok(None);
    }

    let data = unsafe { std::slice::from_raw_parts(ptr, size as usize) };
    let packet = TunnelPacket::parse(data).map_err(|_| TunnelError::InvalidPacket)?;

    unsafe { (api.release_receive_packet)(session, ptr); }
    Ok(Some(packet))
}
```

### Step 4: Packet Write

```rust
fn write_packet(&mut self, packet: TunnelPacket) -> Result<(), TunnelError> {
    let (api, session) = self.api_and_session()?;
    let data = packet.as_bytes();

    let ptr = unsafe { (api.allocate_send_packet)(session, data.len() as u32) };
    if ptr.is_null() {
        return Err(TunnelError::Io);
    }

    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        (api.send_packet)(session, ptr);
    }
    Ok(())
}
```

### Step 5: IOCP Integration

`WintunGetReadWaitEvent()` returns a Win32 `HANDLE` that becomes signaled when packets are available. This handle can be associated with the IOCP backend:

```rust
pub fn read_wait_handle(&self) -> Option<isize> {
    let (api, session) = self.api_and_session().ok()?;
    Some(unsafe { (api.get_read_wait_event)(session) })
}
```

In the client/server loop:
```rust
if let Some(wait_handle) = tun.read_wait_handle() {
    runtime.register_fd(10, wait_handle as i32, FdInterest { readable: true, writable: false })?;
}
```

### Step 6: Cleanup in `Drop`

```rust
impl Drop for WindowsTunAdapter {
    fn drop(&mut self) {
        if let Some(api) = &self.api {
            if let Some(session) = self.session {
                unsafe { (api.end_session)(session); }
            }
            if let Some(adapter) = self.adapter {
                unsafe { (api.close_adapter)(adapter); }
            }
        }
    }
}
```

### Step 7: Struct Changes

```rust
pub struct WindowsTunAdapter {
    name: String,
    mtu: u16,
    opened: bool,
    #[cfg(target_os = "windows")]
    api: Option<WintunApi>,
    #[cfg(target_os = "windows")]
    adapter: Option<WINTUN_ADAPTER_HANDLE>,
    #[cfg(target_os = "windows")]
    session: Option<WINTUN_SESSION_HANDLE>,
    loopback_queue: VecDeque<TunnelPacket>,  // kept for non-Windows and test fallback
}
```

## Testing

| Test                           | Environment       | Method                                                |
|--------------------------------|--------------------|-------------------------------------------------------|
| DLL load failure graceful      | Windows CI         | Rename/remove `wintun.dll`, verify `OpenFailed` error |
| Adapter create + destroy       | Windows CI (admin) | Create adapter, verify name appears in `ipconfig`     |
| Packet roundtrip               | Windows CI (admin) | Write IPv4 packet, read back, verify content          |
| Non-Windows fallback           | macOS/Linux CI     | Verify loopback stub still works (existing test)      |

## CI Changes

`.github/workflows/ci.yml` Windows job needs:

```yaml
- name: Download WinTUN
  run: |
    curl -L -o wintun.zip https://www.wintun.net/builds/wintun-0.14.1.zip
    unzip wintun.zip
    copy wintun\bin\amd64\wintun.dll .

- name: Run tests
  run: cargo test --all-targets
```

## Risk Assessment

| Risk                        | Severity | Mitigation                                                     |
|-----------------------------|----------|----------------------------------------------------------------|
| `wintun.dll` not found      | Medium   | Graceful `TunnelError::OpenFailed`, loopback fallback          |
| Admin privileges required   | High     | Document in operations.md, CI uses admin runner                |
| DLL version mismatch        | Low      | Pin to WinTUN 0.14.x API, check version at load time          |
| Ring buffer full             | Medium   | `AllocateSendPacket` returns NULL, map to `TunnelError::Io`   |
| Memory safety                | High     | All WinTUN calls in minimal unsafe blocks, null checks on every pointer |

## Files to Modify

| File                                       | Change                                              |
|--------------------------------------------|------------------------------------------------------|
| `src/tunnel/tun_windows.rs`                | Full WinTUN implementation behind `#[cfg(windows)]`  |
| `Cargo.toml`                               | Add `Win32_System_LibraryLoader` feature             |
| `.github/workflows/ci.yml`                 | Add WinTUN DLL download step to Windows job          |
| `docs/operations.md`                       | Add WinTUN installation instructions                 |
