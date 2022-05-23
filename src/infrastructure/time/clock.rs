use crate::time::duration::Duration;
use crate::time::time::Clock;

pub struct JSClock {
  previous_time: Option<f64>
}

impl JSClock {
  pub fn new() -> JSClock {
    JSClock {
      previous_time: None
    }
  }

  fn get_milliseconds() -> f64 {
    let window = web_sys::window().expect("Window is not available");
    if let Some(performance) = window.performance() {
      performance.now()
    } else {
      js_sys::Date::now()
    }
  }
}

impl Clock for JSClock {
  fn now(&self) -> Duration {
    let current = JSClock::get_milliseconds();
    let nanoseconds = current * 1_000_000f64;
    Duration {
      nanoseconds: nanoseconds as u128
    }
  }

  fn wait(cycles: u32) {
    todo!()
  }
}