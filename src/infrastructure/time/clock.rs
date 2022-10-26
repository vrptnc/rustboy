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