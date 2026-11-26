//! 일정한 간격의 Tick 일정입니다. 현재 시각은 호출하는 쪽이 넘기므로, 테스트는 가짜 시각을 사용합니다.

use std::time::{Duration, Instant};

/// 서버가 오래 멈췄다가(디버거, 노트북의 절전) 다시 실행되면, 밀린 Tick을 최대 이만큼만 실행하고 나머지는 버립니다.
/// 제한이 없으면 서버가 밀린 Tick을 실행하느라 다시 밀리는 일이 반복됩니다.
const MAX_CATCH_UP: u32 = 5;

pub(crate) struct TickClock {
    period: Duration,
    next_due: Instant,
}

impl TickClock {
    pub fn new(tick_hz: u32, start: Instant) -> TickClock {
        let period = Duration::from_secs(1) / tick_hz;
        TickClock {
            period,
            next_due: start + period,
        }
    }

    /// 지금 실행해야 하는 Tick의 수를 돌려주고, 그만큼 일정을 앞으로 옮깁니다.
    pub fn due(&mut self, now: Instant) -> u32 {
        let mut count = 0;
        while now >= self.next_due && count < MAX_CATCH_UP {
            self.next_due += self.period;
            count += 1;
        }
        if now >= self.next_due {
            self.next_due = now + self.period;
        }
        count
    }

    pub fn time_until_next(&self, now: Instant) -> Duration {
        self.next_due.saturating_duration_since(now)
    }
}
