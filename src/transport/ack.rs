#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckFrame {
    pub largest_ack: u64,
    pub ack_bits: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AckWindow {
    largest_ack: u64,
    ack_bits: u64,
    initialized: bool,
}

impl AckWindow {
    pub fn observe(&mut self, sequence: u64) {
        if !self.initialized {
            self.initialized = true;
            self.largest_ack = sequence;
            self.ack_bits = 1;
            return;
        }

        if sequence > self.largest_ack {
            let shift = (sequence - self.largest_ack).min(63) as u32;
            self.ack_bits <<= shift;
            self.ack_bits |= 1;
            self.largest_ack = sequence;
            return;
        }

        let distance = self.largest_ack - sequence;
        if distance < 64 {
            self.ack_bits |= 1_u64 << distance;
        }
    }

    pub fn is_acked(&self, sequence: u64) -> bool {
        if !self.initialized || sequence > self.largest_ack {
            return false;
        }

        let distance = self.largest_ack - sequence;
        if distance >= 64 {
            return false;
        }

        (self.ack_bits & (1_u64 << distance)) != 0
    }

    pub fn ack_frame(&self) -> Option<AckFrame> {
        if !self.initialized {
            return None;
        }

        Some(AckFrame {
            largest_ack: self.largest_ack,
            ack_bits: self.ack_bits,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::ack::AckWindow;

    #[test]
    fn ack_window_tracks_out_of_order_acks() {
        let mut window = AckWindow::default();
        window.observe(10);
        window.observe(12);
        window.observe(11);

        assert!(window.is_acked(10));
        assert!(window.is_acked(11));
        assert!(window.is_acked(12));
        assert!(!window.is_acked(9));
    }

    #[test]
    fn ack_window_discards_far_history() {
        let mut window = AckWindow::default();
        window.observe(1);
        window.observe(70);

        assert!(!window.is_acked(1));
        assert!(window.is_acked(70));
    }
}
