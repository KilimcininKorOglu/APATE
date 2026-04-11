use crate::transport::mode::AttemptOutcome;
use crate::transport::{Frame, TransportError, TransportStrategy};
use core::time::Duration;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpConnectPolicy {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpTlsTransport {
    connect_policy: TcpConnectPolicy,
    connected: bool,
    outbound: Vec<Frame>,
    inbound: VecDeque<Frame>,
}

impl TcpTlsTransport {
    pub fn new(connect_policy: TcpConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            outbound: Vec::new(),
            inbound: VecDeque::new(),
        }
    }

    pub fn connect(&mut self, _timeout: Duration) -> Result<AttemptOutcome, TransportError> {
        match self.connect_policy {
            TcpConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            TcpConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
        }
    }

    pub fn queue_inbound(&mut self, frame: Frame) {
        self.inbound.push_back(frame);
    }
}

impl TransportStrategy for TcpTlsTransport {
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
