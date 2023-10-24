#[derive(Default)]
pub enum BounceState {
    #[default]
    Reverse,
    Forward,
}

pub struct BounceIterMut<'a, T> {
    slice: *mut [T],
    index: usize,
    bounce_state: BounceState,
    _marker: std::marker::PhantomData<&'a mut [T]>,
}

impl<'a, T> BounceIterMut<'a, T> {
    pub fn new(slice: &'a mut [T]) -> Self {
        Self {
            slice: slice as *mut _,
            index: 0,
            bounce_state: Default::default(),
            _marker: std::marker::PhantomData,
        }
    }
    pub fn new_rev(slice: &'a mut [T]) -> Self {
        let len = slice.len() - 1;
        Self {
            slice: slice as *mut _,
            index: len,
            bounce_state: Default::default(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, T> Iterator for BounceIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: PhantomData locks our lifetime to the lifetime of the array pointer,
        // so use-after-free is impossible
        let len = unsafe { (*self.slice).len() };

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
        let ret = unsafe {
            let slice = &mut *self.slice;
            Some(&mut slice[self.index as usize])
        };

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
        let mut iter = BounceIterMut::new(&mut data);
        assert_eq!(*iter.take(13).map(|x| *x).collect::<Vec<usize>>(), expected);
    }
    #[test]
    fn basic_test_rev() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![5, 4, 3, 2, 1, 2, 3, 4, 5, 4, 3, 2, 1, 2, 3, 4, 5];
        let mut iter = BounceIterMut::new_rev(&mut data);
        assert_eq!(*iter.take(17).map(|x| *x).collect::<Vec<usize>>(), expected);
    }
    #[test]
    fn write() {
        let mut data = vec![1, 2, 3, 4, 5];
        let expected = vec![2, 4, 6, 8, 10];
        let mut iter = BounceIterMut::new(&mut data);
        for item in iter.take(5) {
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
