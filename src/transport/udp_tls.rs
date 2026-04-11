use crate::transport::mode::AttemptOutcome;
use crate::transport::{Frame, TransportError, TransportStrategy};
use core::time::Duration;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpConnectPolicy {
    Success,
    Timeout,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpTlsTransport {
    connect_policy: UdpConnectPolicy,
    connected: bool,
    outbound: Vec<Frame>,
    inbound: VecDeque<Frame>,
}

impl UdpTlsTransport {
    pub fn new(connect_policy: UdpConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            outbound: Vec::new(),
            inbound: VecDeque::new(),
        }
    }

    pub fn connect(&mut self, _timeout: Duration) -> Result<AttemptOutcome, TransportError> {
        match self.connect_policy {
            UdpConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            UdpConnectPolicy::Timeout => Ok(AttemptOutcome::TimedOut),
            UdpConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
        }
    }

    pub fn queue_inbound(&mut self, frame: Frame) {
        self.inbound.push_back(frame);
    }
}

impl TransportStrategy for UdpTlsTransport {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        self.outbound.push(frame);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        Ok(self.inbound.pop_front())
    }
}
