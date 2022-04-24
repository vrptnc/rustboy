use std::cmp::Ordering;
use std::ops;
use crate::time::duration::Duration;

pub trait TimingAware {
  fn tick(&mut self, delta: Duration);
}

#[derive(Copy, Clone)]
pub enum TimeUnit {
  Nanoseconds,
  Microseconds,
  Milliseconds,
  Seconds,
  Minutes,
  Hours,
  Days,
}

impl TimeUnit {
  pub fn to_nanoseconds(&self) -> u64 {
    match self {
      TimeUnit::Nanoseconds => 1,
      TimeUnit::Microseconds => 1_000,
      TimeUnit::Milliseconds => 1_000_000,
      TimeUnit::Seconds => 1_000_000_000,
      TimeUnit::Minutes => 60_000_000_000,
      TimeUnit::Hours => 3600_000_000_000,
      TimeUnit::Days => 86400_000_000_000,
    }
  }

  pub fn min(first: TimeUnit, second: TimeUnit) -> TimeUnit {
    if first.to_nanoseconds() < second.to_nanoseconds() { first } else { second }
  }

  pub fn max(first: TimeUnit, second: TimeUnit) -> TimeUnit {
    if first.to_nanoseconds() > second.to_nanoseconds() { first } else { second }
  }
}

impl PartialEq for TimeUnit {
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl PartialOrd for TimeUnit {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    let nanos = self.to_nanoseconds();
    let other_nanos = other.to_nanoseconds();
    Some(
      if nanos < other_nanos {
        Ordering::Less
      } else if nanos > other_nanos {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    )
  }
}