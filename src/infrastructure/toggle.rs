pub struct Toggle(pub bool);

impl Toggle {
  pub fn toggle(&mut self) -> bool {
    self.0 = !self.0;
    self.0
  }

  pub fn inspect_and_toggle(&mut self) -> bool {
    self.0 = !self.0;
    !self.0
  }

  pub fn inspect_and_clear(&mut self) -> bool {
    let result = self.0;
    self.0 = false;
    result
  }

  pub fn checked(&self) -> bool {
    self.0
  }

  pub fn clear(&mut self) {
    self.0 = false;
  }

  pub fn check(&mut self) {
    self.0 = true;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn toggle() {
    let mut toggle = Toggle(false);
    assert_eq!(toggle.toggle(), true);
    assert_eq!(toggle.checked(), true);
    assert_eq!(toggle.toggle(), false);
    assert_eq!(toggle.checked(), false);
  }

  #[test]
  fn inspect_and_toggle() {
    let mut toggle = Toggle(false);
    assert_eq!(toggle.inspect_and_toggle(), false);
    assert_eq!(toggle.checked(), true);
    assert_eq!(toggle.inspect_and_toggle(), true);
    assert_eq!(toggle.checked(), false);
  }

  #[test]
  fn inspect_and_clear() {
    let mut toggle = Toggle(true);
    assert_eq!(toggle.checked(), true);
    assert_eq!(toggle.inspect_and_clear(), true);
    assert_eq!(toggle.checked(), false);
  }

  #[test]
  fn clear() {
    let mut toggle = Toggle(true);
    assert_eq!(toggle.checked(), true);
    toggle.clear();
    assert_eq!(toggle.checked(), false);
  }

  #[test]
  fn check() {
    let mut toggle = Toggle(false);
    assert_eq!(toggle.checked(), false);
    toggle.check();
    assert_eq!(toggle.checked(), true);
  }
}