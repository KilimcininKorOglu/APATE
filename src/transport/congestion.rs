#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CongestionController {
    state: CongestionState,
    cwnd_bytes: u32,
    ssthresh_bytes: u32,
    mss_bytes: u16,
}

impl CongestionController {
    pub fn new(initial_cwnd_bytes: u32, ssthresh_bytes: u32, mss_bytes: u16) -> Self {
        Self {
            state: CongestionState::SlowStart,
            cwnd_bytes: initial_cwnd_bytes,
            ssthresh_bytes,
            mss_bytes,
        }
    }

    pub fn state(&self) -> CongestionState {
        self.state
    }

    pub fn cwnd_bytes(&self) -> u32 {
        self.cwnd_bytes
    }

    pub fn on_ack(&mut self) {
        match self.state {
            CongestionState::SlowStart => {
                self.cwnd_bytes = self.cwnd_bytes.saturating_add(u32::from(self.mss_bytes));
                if self.cwnd_bytes >= self.ssthresh_bytes {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                let increment = (u32::from(self.mss_bytes) * u32::from(self.mss_bytes))
                    .checked_div(self.cwnd_bytes.max(1))
                    .unwrap_or(1)
                    .max(1);
                self.cwnd_bytes = self.cwnd_bytes.saturating_add(increment);
            }
            CongestionState::Recovery => {
                self.state = CongestionState::CongestionAvoidance;
            }
        }
    }

    pub fn on_loss(&mut self) {
        self.ssthresh_bytes = (self.cwnd_bytes / 2).max(u32::from(self.mss_bytes) * 2);
        self.cwnd_bytes = self.ssthresh_bytes;
        self.state = CongestionState::Recovery;
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::congestion::{CongestionController, CongestionState};

    #[test]
    fn controller_switches_to_avoidance_after_ssthresh() {
        let mut controller = CongestionController::new(1200, 2400, 1200);

        controller.on_ack();

        assert_eq!(CongestionState::CongestionAvoidance, controller.state());
    }

    #[test]
    fn loss_event_enters_recovery_and_reduces_window() {
        let mut controller = CongestionController::new(9600, 9600, 1200);

        controller.on_loss();

        assert_eq!(CongestionState::Recovery, controller.state());
        assert_eq!(4800, controller.cwnd_bytes());
    }
}
