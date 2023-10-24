use std::{rc::Rc, sync::RwLock};

#[derive(Default)]
pub enum BounceState {
    #[default]
    Reverse,
    Forward,
}

// TODO: when I have some caffeine, come up with a better name
pub fn rwlockify<T: Clone>(v: impl Iterator<Item = T>) -> Vec<Rc<RwLock<T>>> {
    v.map(|x| Rc::new(RwLock::new(x.clone()))).collect()
}

pub struct BounceIterMut<T> {
    slice: Vec<Rc<RwLock<T>>>,
    index: usize,
    bounce_state: BounceState,
}

impl<T> BounceIterMut<T> {
    pub fn new(slice: Vec<Rc<RwLock<T>>>) -> Self {
        Self {
            slice,
            index: 0,
            bounce_state: Default::default(),
        }
    }
    pub fn new_rev(slice: Vec<Rc<RwLock<T>>>) -> Self {
        let len = slice.len() - 1;
        Self {
            slice,
            index: len,
            bounce_state: Default::default(),
        }
    }
}

impl<T> Iterator for BounceIterMut<T> {
    type Item = Rc<RwLock<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = (*self.slice).len();

        if self.index >= len {
            self.bounce_state = BounceState::Reverse;
            self.index = len - 2;
        } else if self.index == 0 {
            self.bounce_state = BounceState::Forward;
        }
        // SAFETY: PhantomData locks our lifetime to the lifetime of the array pointer,
        // so use-after-free is impossible
        // TODO: check if multiple mutable ref possible,
        // may undermine safety guarantees in niche ways.
        let ret = Some(self.slice[self.index as usize].clone());

        match self.bounce_state {
            BounceState::Reverse => {
                self.index -= 1;
            }
            BounceState::Forward => {
                self.index += 1;
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    #[test]
    fn basic_test() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterMut::new(rwlockify(data.iter()));
        // assert_eq!(*iter.take(13).collect::<Vec<usize>>(), expected);
    }
    #[test]
    fn basic_test_rev() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![5, 4, 3, 2, 1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterMut::new_rev(rwlockify(data.iter()));
        // assert_eq!(*iter.take(17).map(|x| *x).collect::<Vec<usize>>(), expected);
    }
    #[test]
    fn write() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![2, 4, 6, 8, 10];
        let mut iter = BounceIterMut::new(rwlockify(data.iter()));
        for item in iter.take(5) {
            let item = item.lock();
            let value = *item;
            *item = value * 2;
        }
        assert_eq!(data, expected);
    }
    // CORRECT: Fails due to *mut [i32] not being Send
    // #[test]
    // fn move_to_new_thread() {
    //     let mut data = vec![1, 2, 3, 4, 5];
    //     let mut iter = Arc::new(Mutex::new(BounceIterMut::new(&mut data)));
    //     let iter_ptr = iter.clone();

    //     std::thread::spawn(|| {
    //         iter_ptr.lock().unwrap();
    //     });
    // }
}
