use std::iter::Enumerate;

pub struct SizedIterator<I, T> where I: ExactSizeIterator<Item=T> {
  iterator: Enumerate<I>,
}

impl<I, T> SizedIterator<I, T> where I: ExactSizeIterator<Item=T> {
  pub fn new(iterator: I) -> SizedIterator<I, T> {
    SizedIterator {
      iterator: iterator.enumerate()
    }
  }
}

impl<I, T> Iterator for SizedIterator<I, T> where I: ExactSizeIterator<Item=T> {
  type Item = (T, usize, usize);

  fn next(&mut self) -> Option<Self::Item> {
    if let Some((index, item)) = self.iterator.next() {
      Some((item, index, self.iterator.len()))
    } else {
      None
    }
  }
}