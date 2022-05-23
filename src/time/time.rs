use std::cmp::Ordering;
use std::ops;
use crate::memory::memory::Memory;
use crate::time::duration::Duration;

pub trait ClockAware {
  fn tick(&mut self);
}

pub trait Clock {
  fn wait(cycles: u32);
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
  pub fn to_nanoseconds(&self) -> u128 {
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
}

impl PartialEq for TimeUnit {
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl PartialOrd for TimeUnit {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    let first_discriminant = std::mem::discriminant(self);
    let second_discriminant = std::mem::discriminant(other);
    Some(
      if first_discriminant < second_discriminant {
        Ordering::Less
      } else if first_discriminant > second_discriminant {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    )
  }
}