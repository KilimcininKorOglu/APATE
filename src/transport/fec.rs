use crate::transport::mode::TransportKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FecMode {
    Disabled,
    SingleParity,
    DoubleParity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FecController {
    mode: FecMode,
    single_parity_loss_threshold: f32,
    double_parity_loss_threshold: f32,
}

impl Default for FecController {
    fn default() -> Self {
        Self {
            mode: FecMode::Disabled,
            single_parity_loss_threshold: 0.05,
            double_parity_loss_threshold: 0.20,
        }
    }
}

impl FecController {
    pub fn mode(&self) -> FecMode {
        self.mode
    }

    pub fn update_loss_rate(&mut self, loss_rate: f32, transport_kind: TransportKind) {
        if transport_kind == TransportKind::TcpTls {
            self.mode = FecMode::Disabled;
            return;
        }

        if loss_rate >= self.double_parity_loss_threshold {
            self.mode = FecMode::DoubleParity;
        } else if loss_rate >= self.single_parity_loss_threshold {
            self.mode = FecMode::SingleParity;
        } else {
            self.mode = FecMode::Disabled;
        }
    }

    pub fn parity_shard_count(&self) -> usize {
        match self.mode {
            FecMode::Disabled => 0,
            FecMode::SingleParity => 1,
            FecMode::DoubleParity => 2,
        }
    }

    pub fn build_parity_shards(&self, data_shards: &[Vec<u8>]) -> Vec<Vec<u8>> {
        let parity_count = self.parity_shard_count();
        if parity_count == 0 || data_shards.is_empty() {
            return Vec::new();
        }

        let longest = data_shards.iter().map(Vec::len).max().unwrap_or(0);
        let mut parity = vec![0_u8; longest];
        for shard in data_shards {
            for (index, byte) in shard.iter().copied().enumerate() {
                parity[index] ^= byte;
            }
        }

        if parity_count == 1 {
            return vec![parity];
        }

        let mut secondary = vec![0_u8; longest];
        for shard in data_shards {
            for (index, byte) in shard.iter().copied().enumerate() {
                let rotated = byte.rotate_left((index % 8) as u32);
                secondary[index] ^= rotated;
            }
        }

        vec![parity, secondary]
    }
}

pub fn recover_single_lost_shard(shards: &[Option<Vec<u8>>], parity: &[u8]) -> Option<Vec<u8>> {
    let lost_count = shards.iter().filter(|shard| shard.is_none()).count();
    if lost_count != 1 {
        return None;
    }

    let mut recovered = parity.to_vec();
    for bytes in shards.iter().flatten() {
        for (index, byte) in bytes.iter().copied().enumerate() {
            if index < recovered.len() {
                recovered[index] ^= byte;
            }
        }
    }

    Some(recovered)
}

#[cfg(test)]
mod tests {
    use crate::transport::fec::{FecController, FecMode, recover_single_lost_shard};
    use crate::transport::mode::TransportKind;

    #[test]
    fn fec_controller_adapts_mode_for_loss_levels() {
        let mut controller = FecController::default();
        controller.update_loss_rate(0.03, TransportKind::UdpTls);
        assert_eq!(FecMode::Disabled, controller.mode());

        controller.update_loss_rate(0.07, TransportKind::UdpTls);
        assert_eq!(FecMode::SingleParity, controller.mode());

        controller.update_loss_rate(0.25, TransportKind::UdpTls);
        assert_eq!(FecMode::DoubleParity, controller.mode());
    }

    #[test]
    fn fec_controller_disables_parity_in_tcp_mode() {
        let mut controller = FecController::default();
        controller.update_loss_rate(0.25, TransportKind::TcpTls);
        assert_eq!(FecMode::Disabled, controller.mode());
        assert_eq!(0, controller.parity_shard_count());
    }

    #[test]
    fn single_parity_recovers_lost_shard() {
        let shards = vec![Some(vec![1, 2, 3]), Some(vec![4, 5, 6]), None];
        let parity = vec![1 ^ 4 ^ 7, 2 ^ 5 ^ 8, 3 ^ 6 ^ 9];
        let recovered = recover_single_lost_shard(&shards, &parity).expect("recover");

        assert_eq!(vec![7, 8, 9], recovered);
    }
}
