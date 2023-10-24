use std::{rc::Rc, sync::RwLock};

#[derive(Default)]
pub enum BounceState {
    Reverse,
    #[default]
    Forward,
}

// TODO: when I have some caffeine, come up with a better name
pub fn rwlockify<T: Clone>(iter: impl Iterator<Item = T>) -> impl Iterator<Item = Rc<RwLock<T>>> {
    iter.map(|x| Rc::new(RwLock::new(x.to_owned())))
}

pub fn unrwlockify<T: Clone>(iter: impl Iterator<Item = Rc<RwLock<T>>>) -> impl Iterator<Item = T> {
    iter.map(|x| x.read().expect("Failed to read RWlock").to_owned())
}

pub struct BounceIterMut<T> {
    collection: Vec<Rc<RwLock<T>>>,
    index: usize,
    bounce_state: BounceState,
}

impl<T> BounceIterMut<T> {
    pub fn reset(&mut self) {
        self.index = 0;
    }
    pub fn reset_rev(&mut self) {
        self.index = self.collection.len() - 1;
    }
    pub fn new(collection: Vec<Rc<RwLock<T>>>) -> Self {
        Self {
            collection,
            index: 0,
            bounce_state: Default::default(),
        }
    }
    pub fn new_rev(collection: Vec<Rc<RwLock<T>>>) -> Self {
        let len = collection.len() - 1;
        Self {
            collection,
            index: len,
            bounce_state: Default::default(),
        }
    }
}

impl<T> Iterator for BounceIterMut<T> {
    type Item = Rc<RwLock<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.collection.len();

        if self.index >= len {
            self.bounce_state = BounceState::Reverse;
            self.index = len - 2;
        } else if self.index == 0 {
            self.bounce_state = BounceState::Forward;
        }
        let ret = self.collection[self.index].clone();

        match self.bounce_state {
            BounceState::Reverse => {
                self.index -= 1;
            }
            BounceState::Forward => {
                self.index += 1;
            }
        }
        Some(ret)
    }
}

// #[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn basic_test() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterMut::new(rwlockify(data.iter()).collect());
        assert_eq!(
            *unrwlockify(iter).take(13).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn basic_test_rev() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![5, 4, 3, 2, 1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterMut::new_rev(rwlockify(data.iter()).collect());
        assert_eq!(
            *unrwlockify(iter).take(17).map(|x| *x).collect::<Vec<_>>(),
            expected
        );
    }
    #[test]
    fn write() {
        let ptrs: Vec<_> = rwlockify(vec![1, 2, 3, 4, 5].into_iter()).collect();
        let expected = vec![2, 4, 6, 8, 10];
        let mut iter = BounceIterMut::new(ptrs);
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
}
