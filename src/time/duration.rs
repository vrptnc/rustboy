use std::cmp::Ordering;
use std::cmp::Ordering::Equal;
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

impl RTCDuration {
  pub fn to_duration(&self) -> Duration {
    Duration::from_seconds(self.seconds as u64 +
        (self.minutes as u64) * 60 +
        (self.hours as u64) * 3600 +
        (self.days as u64) * 86400,
    )
  }
}

#[derive(Copy, Clone)]
pub struct Duration {
  pub nanoseconds: u128
}

impl Duration {
  pub fn new() -> Duration {
    Duration {
      nanoseconds: 0
    }
  }

  pub fn from_seconds<T>(seconds: T) -> Duration {
    Duration {
      nanoseconds: (seconds as u128) * 1_000_000_000
    }
  }

  pub fn from_nanoseconds<T>(nanoseconds: T) -> Duration {
    Duration {
      nanoseconds: nanoseconds as u128
    }
  }

  pub fn add(&self, amount: u64, unit: TimeUnit) -> Duration {
    *self + Duration::from_nanoseconds(amount * unit.to_nanoseconds())
  }

  pub fn subtract(&self, amount: u64, unit: TimeUnit) -> Duration {
    *self - Duration::from_nanoseconds(amount * unit.to_nanoseconds())
  }

  pub fn divide_by(&self, amount: u64, unit: TimeUnit) -> u128 {
    *self / Duration::from_nanoseconds(amount * unit.to_nanoseconds())
  }

  pub fn to_rtc_duration(&self) -> RTCDuration {
    let mut seconds = (self.nanoseconds / 1_000_000_000) as u64;
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
    Duration::from_nanoseconds(self.nanoseconds + rhs.nanoseconds)
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
    Duration::from_nanoseconds(self.nanoseconds - rhs.nanoseconds)
  }
}

impl ops::Div<Duration> for Duration {
  type Output = u128;

  fn div(self, rhs: Duration) -> Self::Output {
    self.nanoseconds / rhs.nanoseconds
  }
}

impl PartialEq for Duration {
  fn eq(&self, other: &Self) -> bool {
    self.nanoseconds == other.nanoseconds
  }
}

impl PartialOrd for Duration {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    if self.nanoseconds > other.nanoseconds {
      Some(Ordering::Greater)
    } else if self.nanoseconds < other.nanoseconds {
      Some(Order::Less)
    } else {
      Some(Ordering::Equal)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn duration_add() {}
}