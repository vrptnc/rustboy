use std::ops;
use crate::time::time::TimeUnit;
use crate::time::time::TimeUnit::Nanoseconds;

#[derive(Copy, Clone)]
pub struct RTCDuration {
  pub seconds: u8,
  pub minutes: u8,
  pub hours: u8,
  pub days: u16,
}

#[derive(Copy, Clone)]
pub struct Duration {
  pub nanoseconds: u32,
  pub seconds: u64,
}

impl RTCDuration {
  pub fn to_duration(&self) -> Duration {
    Duration {
      nanoseconds: 0,
      seconds: self.seconds as u64 +
        (self.minutes as u64) * 60 +
        (self.hours as u64) * 3600 +
        (self.days as u64) * 86400,
    }
  }
}

impl Duration {
  pub fn new() -> Duration {
    Duration {
      nanoseconds: 0,
      seconds: 0,
    }
  }

  pub fn from_nanoseconds(nanoseconds: u64) -> Duration {
    Duration {
      nanoseconds: (nanoseconds % 1_000_000_000) as u32,
      seconds: nanoseconds / 1_000_000_000,
    }
  }

  pub fn add(&self, amount: u64, unit: TimeUnit) -> Duration {
    *self + Duration::from_nanoseconds(amount * unit.to_nanoseconds())
  }

  pub fn subtract(&self, amount: u64, unit: TimeUnit) -> Duration {
    *self - Duration::from_nanoseconds(amount * unit.to_nanoseconds())
  }

  pub fn to_rtc_duration(&self) -> RTCDuration {
    let mut seconds = self.seconds;
    let days = seconds / 86400;
    seconds -= days * 86400;
    let hours = seconds / 3600;
    seconds -= hours * 3600;
    let minutes = seconds / 60;
    seconds -= minutes * 60;
    RTCDuration {
      seconds: seconds as u8,
      minutes: minutes as u8,
      hours: hours as u8,
      days: days as u16,
    }
  }
}

impl ops::Add<Duration> for Duration {
  type Output = Duration;

  fn add(self, rhs: Duration) -> Self::Output {
    let total_nanoseconds = self.nanoseconds + rhs.nanoseconds;
    let total_seconds = self.seconds + rhs.seconds + if total_nanoseconds >= 1_000_000_000 { 1 } else { 0 };
    Duration {
      nanoseconds: total_nanoseconds % 1_000_000_000,
      seconds: total_seconds,
    }
  }
}

impl ops::AddAssign<Duration> for Duration {
  fn add_assign(&mut self, other: Duration) {
    *self = *self + other
  }
}

impl ops::Sub<Duration> for Duration {
  type Output = Duration;

  fn sub(self, rhs: Duration) -> Self::Output {
    let (nanoseconds, overflowed) = self.nanoseconds.overflowing_sub(rhs.nanoseconds);
    let seconds = self.seconds - rhs.seconds - if overflowed { 1 } else { 0 };
    Duration {
      nanoseconds,
      seconds,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn duration_add() {}
}