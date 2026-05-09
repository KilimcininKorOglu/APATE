pub struct RingBuf<T> {
    storage: Vec<Option<T>>,
    head: usize,
    len: usize,
}

impl<T> RingBuf<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer capacity must be > 0");
        let mut storage = Vec::with_capacity(capacity);
        storage.resize_with(capacity, || None);
        Self {
            storage,
            head: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    pub fn push_back(&mut self, value: T) -> Option<T> {
        let cap = self.capacity();
        let tail = (self.head + self.len) % cap;

        let evicted = self.storage[tail].take();

        self.storage[tail] = Some(value);

        if evicted.is_some() {
            self.head = (self.head + 1) % cap;
        } else {
            self.len += 1;
        }

        evicted
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.storage[self.head].take();
        self.head = (self.head + 1) % self.capacity();
        self.len -= 1;
        value
    }

    pub fn peek_front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.storage[self.head].as_ref()
    }

    pub fn clear(&mut self) {
        for slot in &mut self.storage {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    pub fn iter(&self) -> RingBufIter<'_, T> {
        RingBufIter {
            ring: self,
            index: 0,
        }
    }
}

pub struct RingBufIter<'a, T> {
    ring: &'a RingBuf<T>,
    index: usize,
}

impl<'a, T> Iterator for RingBufIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.ring.len {
            return None;
        }
        let pos = (self.ring.head + self.index) % self.ring.capacity();
        self.index += 1;
        self.ring.storage[pos].as_ref()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.ring.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for RingBufIter<'a, T> {}

#[cfg(test)]
mod tests {
    use super::RingBuf;

    #[test]
    fn push_pop_basic() {
        let mut ring = RingBuf::new(4);
        assert!(ring.is_empty());

        ring.push_back(10);
        ring.push_back(20);
        ring.push_back(30);

        assert_eq!(3, ring.len());
        assert_eq!(Some(&10), ring.peek_front());
        assert_eq!(Some(10), ring.pop_front());
        assert_eq!(Some(20), ring.pop_front());
        assert_eq!(Some(30), ring.pop_front());
        assert!(ring.is_empty());
    }

    #[test]
    fn eviction_on_overflow() {
        let mut ring = RingBuf::new(3);
        ring.push_back(1);
        ring.push_back(2);
        ring.push_back(3);
        assert!(ring.is_full());

        let evicted = ring.push_back(4);
        assert_eq!(Some(1), evicted);
        assert_eq!(3, ring.len());
        assert_eq!(Some(&2), ring.peek_front());
    }

    #[test]
    fn iter_in_insertion_order() {
        let mut ring = RingBuf::new(4);
        ring.push_back(100);
        ring.push_back(200);
        ring.push_back(300);

        let collected: Vec<_> = ring.iter().copied().collect();
        assert_eq!(vec![100, 200, 300], collected);
    }

    #[test]
    fn clear_resets_state() {
        let mut ring = RingBuf::new(4);
        ring.push_back(1);
        ring.push_back(2);
        ring.clear();

        assert!(ring.is_empty());
        assert_eq!(None, ring.pop_front());
    }

    #[test]
    fn wraparound_ordering() {
        let mut ring = RingBuf::new(3);
        ring.push_back(1);
        ring.push_back(2);
        ring.push_back(3);
        ring.pop_front();
        ring.pop_front();
        ring.push_back(4);
        ring.push_back(5);

        let collected: Vec<_> = ring.iter().copied().collect();
        assert_eq!(vec![3, 4, 5], collected);
    }
}
