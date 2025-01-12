use std::time::Instant;

pub struct SwapIter<'a, T> {
    vec: &'a mut Vec<T>,
    index: usize
}

impl<'a, T> SwapIter<'a, T> {
    pub fn new(collection: &'a mut Vec<T>) -> Self {
        Self {
            vec: collection,
            index: 0,
        }
    }

    pub fn next(&mut self) -> (&mut Vec<T>, T) {
        let element = self.vec.swap_remove(self.index);

        (&mut self.vec, element)

        // dont increment to the next index because we just removed an element
    }

    pub fn restore(&mut self, element: T) {
        
        // return the element to the vector
        self.vec.push(element);

        // swap the restored element back to its original position
        let len = self.vec.len();
        self.vec.swap(len - 1, self.index);

        self.index += 1;
    }

    pub fn not_done(&self) -> bool {
        self.index < self.vec.len()
    }
}
