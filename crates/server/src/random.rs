//! 서버가 새 방의 코드와 자리의 표식을 만들 때 쓰는 무작위 바이트의 원천입니다.
//!
//! 서버는 운영체제를 직접 읽지 않으므로, 호출하는 쪽이 원천을 넘깁니다.
//! 전용 서버는 운영체제의 암호용 무작위를 넘기고, 테스트는 같은 값을 다시 만드는 `PredictableRandom`을 넘깁니다.

/// 무작위 바이트를 주는 곳입니다.
pub trait RandomSource {
    /// `bytes`를 무작위 바이트로 채웁니다. 자리의 표식은 이 바이트가 그대로 되고, 표식을 아는 사람은 그 자리를 가져갈 수 있습니다.
    /// 그래서 실제 서버의 원천은 다른 사람이 다음 값을 예측할 수 없는 암호용 무작위여야 합니다.
    fn fill(&mut self, bytes: &mut [u8]);
}

/// 다음 값을 예측할 수 있는 무작위입니다. 같은 `seed`에서는 늘 같은 바이트가 나오므로, 테스트에서 같은 방 코드와 표식을 다시 만드는 데 씁니다.
/// 표식이 비밀이어야 하는 실제 서버에는 쓰지 않습니다.
pub struct PredictableRandom {
    state: u64,
}

impl PredictableRandom {
    pub fn new(seed: u64) -> PredictableRandom {
        PredictableRandom { state: seed }
    }

    /// 64비트 값 하나를 만듭니다(SplitMix64).
    fn next_word(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut word = self.state;
        word = (word ^ (word >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        word ^ (word >> 31)
    }
}

impl RandomSource for PredictableRandom {
    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let word = self.next_word().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_of(seed: u64, count: usize) -> Vec<[u8; 16]> {
        let mut random = PredictableRandom::new(seed);
        (0..count)
            .map(|_| {
                let mut bytes = [0u8; 16];
                random.fill(&mut bytes);
                bytes
            })
            .collect()
    }

    #[test]
    fn the_same_seed_gives_the_same_bytes_and_another_seed_gives_other_bytes() {
        assert_eq!(bytes_of(1, 5), bytes_of(1, 5));
        assert_ne!(bytes_of(1, 5), bytes_of(2, 5));
    }

    #[test]
    fn each_fill_gives_new_bytes_and_any_length_is_filled() {
        let draws = bytes_of(7, 50);
        let distinct: std::collections::BTreeSet<_> = draws.iter().collect();
        assert_eq!(distinct.len(), draws.len());

        let mut random = PredictableRandom::new(7);
        for length in [0, 1, 7, 8, 9, 31] {
            let mut bytes = vec![0u8; length];
            random.fill(&mut bytes);
            assert!(
                length < 8 || bytes.iter().any(|&byte| byte != 0),
                "{length} bytes were filled"
            );
        }
    }
}
