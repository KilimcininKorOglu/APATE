use crate::stealth::entropy::EntropySource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficState {
    Idle,
    PageLoad,
    Streaming,
    Interactive,
    BurstDownload,
}

#[derive(Debug, Clone)]
pub struct SizeBucket {
    pub min: u16,
    pub max: u16,
    pub weight: u16,
}

#[derive(Debug, Clone)]
pub struct DelayBucket {
    pub min_us: u64,
    pub max_us: u64,
    pub weight: u16,
}

#[derive(Debug, Clone)]
pub struct StateProfile {
    pub size_buckets: Vec<SizeBucket>,
    pub delay_buckets: Vec<DelayBucket>,
    pub min_packets: u32,
    pub max_packets: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Transition {
    pub to: TrafficState,
    pub weight: u16,
}

#[derive(Debug, Clone)]
pub struct TrafficProfile {
    pub name: String,
    pub idle: StateProfile,
    pub page_load: StateProfile,
    pub streaming: StateProfile,
    pub interactive: StateProfile,
    pub burst_download: StateProfile,
    pub transitions: Vec<(TrafficState, Vec<Transition>)>,
}

impl TrafficProfile {
    fn state_profile(&self, state: TrafficState) -> &StateProfile {
        match state {
            TrafficState::Idle => &self.idle,
            TrafficState::PageLoad => &self.page_load,
            TrafficState::Streaming => &self.streaming,
            TrafficState::Interactive => &self.interactive,
            TrafficState::BurstDownload => &self.burst_download,
        }
    }

    fn transitions_from(&self, state: TrafficState) -> &[Transition] {
        for (from, trans) in &self.transitions {
            if *from == state {
                return trans;
            }
        }
        &[]
    }

    pub fn chrome_h3() -> Self {
        Self {
            name: String::from("chrome_h3"),
            idle: StateProfile {
                size_buckets: vec![
                    SizeBucket {
                        min: 40,
                        max: 100,
                        weight: 70,
                    },
                    SizeBucket {
                        min: 100,
                        max: 300,
                        weight: 30,
                    },
                ],
                delay_buckets: vec![
                    DelayBucket {
                        min_us: 200_000,
                        max_us: 1_000_000,
                        weight: 40,
                    },
                    DelayBucket {
                        min_us: 1_000_000,
                        max_us: 5_000_000,
                        weight: 50,
                    },
                    DelayBucket {
                        min_us: 5_000_000,
                        max_us: 30_000_000,
                        weight: 10,
                    },
                ],
                min_packets: 1,
                max_packets: 5,
            },
            page_load: StateProfile {
                size_buckets: vec![
                    SizeBucket {
                        min: 40,
                        max: 200,
                        weight: 15,
                    },
                    SizeBucket {
                        min: 200,
                        max: 800,
                        weight: 35,
                    },
                    SizeBucket {
                        min: 800,
                        max: 1400,
                        weight: 40,
                    },
                    SizeBucket {
                        min: 1400,
                        max: 1500,
                        weight: 10,
                    },
                ],
                delay_buckets: vec![
                    DelayBucket {
                        min_us: 50,
                        max_us: 500,
                        weight: 25,
                    },
                    DelayBucket {
                        min_us: 500,
                        max_us: 5_000,
                        weight: 40,
                    },
                    DelayBucket {
                        min_us: 5_000,
                        max_us: 50_000,
                        weight: 25,
                    },
                    DelayBucket {
                        min_us: 50_000,
                        max_us: 200_000,
                        weight: 10,
                    },
                ],
                min_packets: 20,
                max_packets: 200,
            },
            streaming: StateProfile {
                size_buckets: vec![
                    SizeBucket {
                        min: 40,
                        max: 100,
                        weight: 10,
                    },
                    SizeBucket {
                        min: 1200,
                        max: 1400,
                        weight: 70,
                    },
                    SizeBucket {
                        min: 1400,
                        max: 1500,
                        weight: 20,
                    },
                ],
                delay_buckets: vec![
                    DelayBucket {
                        min_us: 5_000,
                        max_us: 20_000,
                        weight: 60,
                    },
                    DelayBucket {
                        min_us: 20_000,
                        max_us: 50_000,
                        weight: 30,
                    },
                    DelayBucket {
                        min_us: 50_000,
                        max_us: 100_000,
                        weight: 10,
                    },
                ],
                min_packets: 50,
                max_packets: 500,
            },
            interactive: StateProfile {
                size_buckets: vec![
                    SizeBucket {
                        min: 40,
                        max: 200,
                        weight: 40,
                    },
                    SizeBucket {
                        min: 200,
                        max: 600,
                        weight: 40,
                    },
                    SizeBucket {
                        min: 600,
                        max: 1200,
                        weight: 20,
                    },
                ],
                delay_buckets: vec![
                    DelayBucket {
                        min_us: 10_000,
                        max_us: 100_000,
                        weight: 50,
                    },
                    DelayBucket {
                        min_us: 100_000,
                        max_us: 500_000,
                        weight: 35,
                    },
                    DelayBucket {
                        min_us: 500_000,
                        max_us: 2_000_000,
                        weight: 15,
                    },
                ],
                min_packets: 5,
                max_packets: 50,
            },
            burst_download: StateProfile {
                size_buckets: vec![
                    SizeBucket {
                        min: 40,
                        max: 100,
                        weight: 5,
                    },
                    SizeBucket {
                        min: 1400,
                        max: 1500,
                        weight: 95,
                    },
                ],
                delay_buckets: vec![
                    DelayBucket {
                        min_us: 10,
                        max_us: 200,
                        weight: 70,
                    },
                    DelayBucket {
                        min_us: 200,
                        max_us: 2_000,
                        weight: 30,
                    },
                ],
                min_packets: 100,
                max_packets: 1000,
            },
            transitions: vec![
                (
                    TrafficState::Idle,
                    vec![
                        Transition {
                            to: TrafficState::PageLoad,
                            weight: 70,
                        },
                        Transition {
                            to: TrafficState::Interactive,
                            weight: 20,
                        },
                        Transition {
                            to: TrafficState::Idle,
                            weight: 10,
                        },
                    ],
                ),
                (
                    TrafficState::PageLoad,
                    vec![
                        Transition {
                            to: TrafficState::Streaming,
                            weight: 30,
                        },
                        Transition {
                            to: TrafficState::Interactive,
                            weight: 25,
                        },
                        Transition {
                            to: TrafficState::BurstDownload,
                            weight: 10,
                        },
                        Transition {
                            to: TrafficState::Idle,
                            weight: 35,
                        },
                    ],
                ),
                (
                    TrafficState::Streaming,
                    vec![
                        Transition {
                            to: TrafficState::Idle,
                            weight: 20,
                        },
                        Transition {
                            to: TrafficState::Interactive,
                            weight: 15,
                        },
                        Transition {
                            to: TrafficState::Streaming,
                            weight: 65,
                        },
                    ],
                ),
                (
                    TrafficState::Interactive,
                    vec![
                        Transition {
                            to: TrafficState::PageLoad,
                            weight: 40,
                        },
                        Transition {
                            to: TrafficState::Idle,
                            weight: 40,
                        },
                        Transition {
                            to: TrafficState::Interactive,
                            weight: 20,
                        },
                    ],
                ),
                (
                    TrafficState::BurstDownload,
                    vec![
                        Transition {
                            to: TrafficState::Idle,
                            weight: 50,
                        },
                        Transition {
                            to: TrafficState::Interactive,
                            weight: 30,
                        },
                        Transition {
                            to: TrafficState::PageLoad,
                            weight: 20,
                        },
                    ],
                ),
            ],
        }
    }

    pub fn firefox_h3() -> Self {
        let mut profile = Self::chrome_h3();
        profile.name = String::from("firefox_h3");
        profile.page_load.size_buckets = vec![
            SizeBucket {
                min: 40,
                max: 200,
                weight: 20,
            },
            SizeBucket {
                min: 200,
                max: 700,
                weight: 30,
            },
            SizeBucket {
                min: 700,
                max: 1300,
                weight: 35,
            },
            SizeBucket {
                min: 1300,
                max: 1500,
                weight: 15,
            },
        ];
        profile
    }
}

pub struct ShapedPacket {
    pub payload: Vec<u8>,
    pub delay_us: u64,
    pub state: TrafficState,
}

pub struct TrafficShapingEngine {
    profile: TrafficProfile,
    current_state: TrafficState,
    entropy: EntropySource,
    packets_in_state: u32,
    state_packet_limit: u32,
}

impl TrafficShapingEngine {
    pub fn new(profile: TrafficProfile) -> Self {
        let mut entropy = EntropySource::new();
        let sp = profile.state_profile(TrafficState::Idle);
        let limit = if sp.max_packets > sp.min_packets {
            sp.min_packets
                + entropy.random_in_range(0, (sp.max_packets - sp.min_packets) as u16) as u32
        } else {
            sp.min_packets
        };
        Self {
            profile,
            current_state: TrafficState::Idle,
            entropy,
            packets_in_state: 0,
            state_packet_limit: limit,
        }
    }

    pub fn from_profile_name(name: &str) -> Self {
        let profile = match name {
            "firefox_h3" => TrafficProfile::firefox_h3(),
            _ => TrafficProfile::chrome_h3(),
        };
        Self::new(profile)
    }

    pub fn current_state(&self) -> TrafficState {
        self.current_state
    }

    pub fn shape_packet(&mut self, payload: &[u8]) -> ShapedPacket {
        let size_buckets = self
            .profile
            .state_profile(self.current_state)
            .size_buckets
            .clone();
        let delay_buckets = self
            .profile
            .state_profile(self.current_state)
            .delay_buckets
            .clone();

        let target_size = self.sample_size(&size_buckets);
        let padded = self.pad_payload(payload, target_size as usize);

        let delay_us = self.sample_delay(&delay_buckets);

        self.packets_in_state += 1;
        if self.packets_in_state >= self.state_packet_limit {
            self.evaluate_transition();
        }

        ShapedPacket {
            payload: padded,
            delay_us,
            state: self.current_state,
        }
    }

    pub fn should_send_chaff(&mut self) -> Option<ShapedPacket> {
        if self.current_state != TrafficState::Idle {
            return None;
        }
        let roll = self.entropy.random_in_range(0, 100);
        if roll < 15 {
            let size_buckets = self
                .profile
                .state_profile(TrafficState::Idle)
                .size_buckets
                .clone();
            let delay_buckets = self
                .profile
                .state_profile(TrafficState::Idle)
                .delay_buckets
                .clone();
            let size = self.sample_size(&size_buckets) as usize;
            let mut chaff = vec![0u8; size];
            self.entropy.fill_padding(&mut chaff);
            let delay = self.sample_delay(&delay_buckets);
            Some(ShapedPacket {
                payload: chaff,
                delay_us: delay,
                state: TrafficState::Idle,
            })
        } else {
            None
        }
    }

    fn sample_size(&mut self, buckets: &[SizeBucket]) -> u16 {
        let total_weight: u32 = buckets.iter().map(|b| u32::from(b.weight)).sum();
        if total_weight == 0 {
            return 100;
        }
        let roll = self.entropy.random_in_range(0, (total_weight - 1) as u16) as u32;
        let mut acc = 0u32;
        for bucket in buckets {
            acc += u32::from(bucket.weight);
            if roll < acc {
                return self.entropy.random_in_range(bucket.min, bucket.max);
            }
        }
        buckets.last().map_or(100, |b| b.max)
    }

    fn sample_delay(&mut self, buckets: &[DelayBucket]) -> u64 {
        let total_weight: u32 = buckets.iter().map(|b| u32::from(b.weight)).sum();
        if total_weight == 0 {
            return 1000;
        }
        let roll = self.entropy.random_in_range(0, (total_weight - 1) as u16) as u32;
        let mut acc = 0u32;
        for bucket in buckets {
            acc += u32::from(bucket.weight);
            if roll < acc {
                return self.entropy.random_delay_us(bucket.min_us, bucket.max_us);
            }
        }
        buckets.last().map_or(1000, |b| b.max_us)
    }

    fn pad_payload(&mut self, payload: &[u8], target_size: usize) -> Vec<u8> {
        let actual_size = target_size.max(payload.len());
        let mut result = Vec::with_capacity(actual_size);
        result.extend_from_slice(payload);
        if result.len() < actual_size {
            let pad_len = actual_size - result.len();
            let mut padding = vec![0u8; pad_len];
            self.entropy.fill_padding(&mut padding);
            result.extend_from_slice(&padding);
        }
        result
    }

    fn evaluate_transition(&mut self) {
        let transitions = self.profile.transitions_from(self.current_state);
        if transitions.is_empty() {
            self.packets_in_state = 0;
            return;
        }

        let total_weight: u32 = transitions.iter().map(|t| u32::from(t.weight)).sum();
        if total_weight == 0 {
            return;
        }

        let roll = self.entropy.random_in_range(0, (total_weight - 1) as u16) as u32;
        let mut acc = 0u32;
        let mut next_state = self.current_state;
        for t in transitions {
            acc += u32::from(t.weight);
            if roll < acc {
                next_state = t.to;
                break;
            }
        }

        self.current_state = next_state;
        self.packets_in_state = 0;

        let sp = self.profile.state_profile(next_state);
        self.state_packet_limit = if sp.max_packets > sp.min_packets {
            sp.min_packets
                + self
                    .entropy
                    .random_in_range(0, (sp.max_packets - sp.min_packets) as u16)
                    as u32
        } else {
            sp.min_packets
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_profile_has_all_states() {
        let profile = TrafficProfile::chrome_h3();
        assert!(!profile.idle.size_buckets.is_empty());
        assert!(!profile.page_load.size_buckets.is_empty());
        assert!(!profile.streaming.size_buckets.is_empty());
        assert!(!profile.interactive.size_buckets.is_empty());
        assert!(!profile.burst_download.size_buckets.is_empty());
    }

    #[test]
    fn engine_starts_in_idle() {
        let engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        assert_eq!(TrafficState::Idle, engine.current_state());
    }

    #[test]
    fn shape_packet_pads_small_payload() {
        let mut engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        let shaped = engine.shape_packet(&[1, 2, 3]);
        assert!(shaped.payload.len() >= 3);
        assert_eq!(&shaped.payload[..3], &[1, 2, 3]);
    }

    #[test]
    fn shape_packet_preserves_large_payload() {
        let mut engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        let large = vec![0xAB; 1500];
        let shaped = engine.shape_packet(&large);
        assert!(shaped.payload.len() >= 1500);
        assert_eq!(&shaped.payload[..1500], &large[..]);
    }

    #[test]
    fn engine_transitions_state_after_packets() {
        let mut engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        let initial = engine.current_state();

        for _ in 0..1000 {
            engine.shape_packet(&[0; 100]);
        }

        let transitioned = engine.current_state() != initial || engine.packets_in_state < 1000;
        assert!(transitioned);
    }

    #[test]
    fn delay_is_within_profile_bounds() {
        let mut engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        for _ in 0..100 {
            let shaped = engine.shape_packet(&[0; 50]);
            assert!(shaped.delay_us <= 30_000_000);
        }
    }

    #[test]
    fn size_distribution_covers_expected_range() {
        let mut engine = TrafficShapingEngine::from_profile_name("chrome_h3");
        engine.current_state = TrafficState::PageLoad;
        engine.packets_in_state = 0;
        engine.state_packet_limit = 10000;

        let mut sizes: Vec<usize> = Vec::new();
        for _ in 0..1000 {
            let shaped = engine.shape_packet(&[0; 10]);
            sizes.push(shaped.payload.len());
        }

        let small = sizes.iter().filter(|&&s| s < 200).count();
        let medium = sizes.iter().filter(|&&s| (200..800).contains(&s)).count();
        let large = sizes.iter().filter(|&&s| s >= 800).count();

        assert!(small > 50);
        assert!(medium > 100);
        assert!(large > 100);
    }

    #[test]
    fn firefox_profile_differs_from_chrome() {
        let chrome = TrafficProfile::chrome_h3();
        let firefox = TrafficProfile::firefox_h3();
        assert_ne!(chrome.name, firefox.name);
    }
}
