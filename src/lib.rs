use std::{rc::Rc, sync::RwLock};

#[derive(Default)]
pub enum BounceState {
    Reverse,
    #[default]
    Forward,
    NoBounce,
}

// TODO: when I have some caffeine, come up with a better name
pub fn rwlockify<T: Clone>(iter: impl Iterator<Item = T>) -> impl Iterator<Item = Rc<RwLock<T>>> {
    iter.map(|x| Rc::new(RwLock::new(x.to_owned())))
}

// TODO: panic is rude
pub fn unrwlockify<T: Clone>(iter: impl Iterator<Item = Rc<RwLock<T>>>) -> impl Iterator<Item = T> {
    iter.map(|x| x.read().expect("Failed to read RWlock").to_owned())
}

pub struct BounceIterLockedMut<T> {
    collection: Vec<T>,
    index: usize,
    bounce_state: BounceState,
}

// TODO: builtin peekability
// composibility on uniquely featured iterators is somewhat poor,
// so we must implement this ourselves
impl<T> BounceIterLockedMut<T>
where
    T: Clone,
{
    pub fn reset(&mut self) {
        self.index = 0;
    }
    pub fn reset_rev(&mut self) {
        self.index = self.collection.len() - 1;
    }
    pub fn new(collection: Vec<T>) -> Self {
        Self {
            collection,
            index: 0,
            bounce_state: Default::default(),
        }
    }
    pub fn new_rev(collection: Vec<T>) -> Self {
        let len = collection.len() - 1;
        Self {
            collection,
            index: len,
            bounce_state: Default::default(),
        }
    }

    pub fn peek_before(&self) -> Option<T> {
        self.collection.get(self.index - 1).cloned()
    }
    pub fn peek_after(&self) -> Option<T> {
        self.collection.get(self.index + 1).cloned()
    }
}

impl<T> Iterator for BounceIterLockedMut<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.collection.len();
        if len > 1 {
            if self.index >= len {
                self.bounce_state = BounceState::Reverse;
                self.index = len - 2;
            } else if self.index == 0 {
                self.bounce_state = BounceState::Forward;
            }
        } else {
            self.bounce_state = BounceState::NoBounce;
        }
        let ret = self.collection.get(self.index).cloned();

        match self.bounce_state {
            BounceState::Reverse => {
                self.index -= 1;
            }
            BounceState::Forward => {
                self.index += 1;
            }
            BounceState::NoBounce => {}
        }
        ret
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn empty() {
        let mut data: Vec<i32> = Vec::new();
        let mut iter = BounceIterLockedMut::new(rwlockify(data.iter()).collect());
        let expected: Vec<i32> = Vec::new();
        assert_eq!(
            *unrwlockify(iter).take(5).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn smol() {
        let mut data = vec![1];
        let mut iter = BounceIterLockedMut::new(rwlockify(data.iter()).collect());
        let expected = vec![1, 1, 1, 1, 1];
        assert_eq!(
            *unrwlockify(iter).take(5).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn basic_test() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterLockedMut::new(rwlockify(data.iter()).collect());
        assert_eq!(
            *unrwlockify(iter).take(13).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn basic_test_rev() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![5, 4, 3, 2, 1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterLockedMut::new_rev(rwlockify(data.iter()).collect());
        assert_eq!(
            *unrwlockify(iter).take(17).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn write() {
        let ptrs: Vec<_> = rwlockify(vec![1, 2, 3, 4, 5].into_iter()).collect();
        let expected = vec![2, 4, 6, 8, 10];
        let mut iter = BounceIterLockedMut::new(ptrs);
        for _ in 0..5 {
            let Some(item) = iter.next() else {
                break;
            };
            dbg!(&item);
            let value = *item.read().unwrap();
            *item.write().unwrap() = value * 2;
            dbg!(&item);
        }
        iter.reset();
        let data = unrwlockify(iter).take(5).collect::<Vec<_>>();
        assert_eq!(data, expected);
    }
    #[test]
    fn write_multiple() {
        let ptrs: Vec<_> = rwlockify(vec![1, 2, 3, 4, 5].into_iter()).collect();
        // is backward because it occurs after a bounce, and enabling peeking prevents us from calling reset()
        let expected = vec![5, 10, 10, 2, 10];
        let mut iter = BounceIterLockedMut::new(ptrs).peekable();
        for _ in 0..5 {
            let Some(item) = iter.next() else {
                break;
            };
            // 2 is skipped as this applies to the NEXT input
            let peek = iter.peek_mut().unwrap();
            *peek.write().unwrap() = 5;
            dbg!(&item);
            let value = *item.read().unwrap();
            *item.write().unwrap() = value * 2;
            dbg!(&item);
        }
        let data = unrwlockify(iter).take(5).collect::<Vec<_>>();
        assert_eq!(data, expected);
    }
}
