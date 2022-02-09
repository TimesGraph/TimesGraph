use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::{ExactSizeIterator, FusedIterator, TrustedLen};
use tg_core::murmur3::Murmur3;
use tg_mem::alloc::{Hold, Holder, HoldError, Stow, TryClone, CloneIntoHold};
use crate::hash_trie::{HashTrie, HashTrieIter};

/// Hash array mapped trie map.
pub struct HashTrieMap<'a, K, V, H = Murmur3> {
    trie: HashTrie<'a, K, V, H>,
}

/// Iterator over the leafs of a `HashTrieMap`.
pub struct HashTrieMapIter<'a, K: 'a, V: 'a> {
    iter: HashTrieIter<'a, K, V>
}

/// Mutable iterator over the leafs of a `HashTrieMap`.
pub struct HashTrieMapIterMut<'a, K: 'a, V: 'a> {
    iter: HashTrieIter<'a, K, V>
}

/// Iterator over the keys of a `HashTrieMap`.
pub struct HashTrieMapKeys<'a, K: 'a, V: 'a> {
    iter: HashTrieIter<'a, K, V>,
}

/// Iterator over the values of a `HashTrieMap`.
pub struct HashTrieMapVals<'a, K: 'a, V: 'a> {
    iter: HashTrieIter<'a, K, V>,
}

/// Mutabke iterator over the values of a `HashTrieMap`.
pub struct HashTrieMapValsMut<'a, K: 'a, V: 'a> {
    iter: HashTrieIter<'a, K, V>
}

impl<K, V> HashTrieMap<'static, K, V> {
    /// Constructs a new `HashTrieMap` that will allocate its data in the
    /// global `Hold`.
    #[inline]
    pub fn new() -> HashTrieMap<'static, K, V> {
        HashTrieMap::hold_new(Hold::global())
    }
}

impl<K, V, H> HashTrieMap<'static, K, V, H> {
    /// Constructs a new `HashTrieMap` that will allocate its data in the
    /// global `Hold`, and hash its keys using the supplied `hasher`.
    #[inline]
    pub fn new_hasher(hasher: H) -> HashTrieMap<'static, K, V, H> {
        HashTrieMap::hold_new_hasher(Hold::global(), hasher)
    }
}

impl<'a, K, V> HashTrieMap<'a, K, V> {
    /// Constructs a new `HashTrieMap` that will allocate its data in `Hold`.
    /// Allocates a zero-sized root block in `hold`, which typically returns a
    /// shared sentinel pointer to the hold, consuming no additional memory.
    #[inline]
    pub fn hold_new(hold: &dyn Hold<'a>) -> HashTrieMap<'a, K, V> {
        HashTrieMap { trie: HashTrie::hold_new(hold) }
    }
}

impl<'a, K, V, H> HashTrieMap<'a, K, V, H> {
    /// Constructs a new `HashTrieMap` that will allocate its data in `Hold`,
    /// and hash its keys using the supplied `hasher`. Allocates a zero-sized
    /// root block in `hold`, which typically returns a shared sentinel pointer
    /// to the hold, consuming no additional memory.
    #[inline]
    pub fn hold_new_hasher(hold: &dyn Hold<'a>, hasher: H) -> HashTrieMap<'a, K, V, H> {
        HashTrieMap { trie: HashTrie::hold_new_hasher(hold, hasher) }
    }

    /// Returns `true` if this `HashTrieMap` contains no leafs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trie.is_empty()
    }

    /// Returns the number of leafs in this `HashTrieMap`.
    #[inline]
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns an iterator over the leafs of this `HashTrieMap`.
    pub fn iter(&self) -> HashTrieMapIter<'a, K, V> {
        HashTrieMapIter { iter: self.trie.iterator() }
    }

    /// Returns a mutable iterator over the leafs of this `HashTrieMap`.
    pub fn iter_mut(&mut self) -> HashTrieMapIterMut<'a, K, V> {
        HashTrieMapIterMut { iter: self.trie.iterator() }
    }

    /// Returns an iterator over the keys of this `HashTrieMap`.
    pub fn keys(&self) -> HashTrieMapKeys<'a, K, V> {
        HashTrieMapKeys { iter: self.trie.iterator() }
    }

    /// Returns an iterator over the values of this `HashTrieMap`.
    pub fn values(&self) -> HashTrieMapVals<'a, K, V> {
        HashTrieMapVals { iter: self.trie.iterator() }
    }

    /// Returns a mutable iterator over the values of this `HashTrieMap`.
    pub fn values_mut(&mut self) -> HashTrieMapValsMut<'a, K, V> {
        HashTrieMapValsMut { iter: self.trie.iterator() }
    }
}

impl<'a, K: Eq + Hash, V, H: BuildHasher> HashTrieMap<'a, K, V, H> {
    /// Returns `true` if this `HashTrieMap` contains the given `key`.
    pub fn contains_key<J: Borrow<K> + ?Sized>(&self, key: &J) -> bool {
        self.trie.contains_key(key)
    }

    /// Returns the value associated with the given `key`, or `None` if no
    /// association exists.
    pub fn get<J: Borrow<K> + ?Sized>(&self, key: &J) -> Option<&V> {
        self.trie.get(key)
    }

    /// Associates a new `value` with the given `key`; returns the previous
    /// value associated with the `key`, if defined. If the trie's `Hold` fails
    /// to allocate any required new memory, returns the `key` and `value`,
    /// along with a `HoldError`, and leaves the trie in its original state.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V, HoldError)> {
        self.trie.insert(key, value)
    }

    /// Disassociates the given `key`; returns the previous value associated
    /// with the `key`, if any. Returns a `HoldError`, and leaves the trie in
    /// its original state, if the trie's `Hold` fails to allocate any required
    /// new memory.
    pub fn remove<J: Borrow<K> + ?Sized>(&mut self, key: &J) -> Result<Option<V>, HoldError> {
        self.trie.remove(key)
    }
}

impl<'a, K, V, H> Holder<'a> for HashTrieMap<'a, K, V, H> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        self.trie.holder()
    }
}

impl<'a, K: Clone, V: Clone, H: Clone> TryClone for HashTrieMap<'a, K, V, H> {
    fn try_clone(&self) -> Result<HashTrieMap<'a, K, V, H>, HoldError> {
        Ok(HashTrieMap { trie: self.trie.try_clone()? })
    }
}

impl<'a, K: Clone, V: Clone, H: Clone> CloneIntoHold<'a, HashTrieMap<'a, K, V, H>> for HashTrieMap<'a, K, V, H> {
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<HashTrieMap<'a, K, V, H>, HoldError> {
        Ok(HashTrieMap { trie: self.trie.try_clone_into_hold(hold)? })
    }
}

impl<'a, 'b, K, V, H> Stow<'b, HashTrieMap<'b, K, V, H>> for HashTrieMap<'a, K, V, H>
    where K: Stow<'b>,
          V: Stow<'b>,
          H: Stow<'b>,
{
    unsafe fn stow(src: *mut HashTrieMap<'a, K, V, H>, dst: *mut HashTrieMap<'b, K, V, H>, hold: &Hold<'b>)
        -> Result<(), HoldError>
    {
        HashTrie::stow(&mut (*src).trie, &mut (*dst).trie, hold)?;
        Ok(())
    }

    unsafe fn unstow(src: *mut HashTrieMap<'a, K, V, H>, dst: *mut HashTrieMap<'b, K, V, H>) {
        HashTrie::unstow(&mut (*src).trie, &mut (*dst).trie);
    }
}

impl<'a, K, V, H> IntoIterator for &'a HashTrieMap<'a, K, V, H> {
    type Item = (&'a K, &'a V);
    type IntoIter = HashTrieMapIter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> HashTrieMapIter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V, H> IntoIterator for &'a mut HashTrieMap<'a, K, V, H> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = HashTrieMapIterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> HashTrieMapIterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<'a, K: 'a + fmt::Debug, V: 'a + fmt::Debug> fmt::Debug for HashTrieMap<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for HashTrieMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        unsafe {
            match self.iter.next() {
                Some(leaf) => Some((&(*leaf.as_ptr()).0, &(*leaf.as_ptr()).1)),
                None => None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for HashTrieMapIter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some((&(*leaf.as_ptr()).0, &(*leaf.as_ptr()).1)),
                None => None,
            }
        }
    }
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for HashTrieMapIter<'a, K, V> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for HashTrieMapIter<'a, K, V> {
}

unsafe impl<'a, K: 'a, V: 'a> TrustedLen for HashTrieMapIter<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieMapIter<'a, K, V> {
    fn clone(&self) -> HashTrieMapIter<'a, K, V> {
        HashTrieMapIter { iter: self.iter.clone() }
    }
}

impl<'a, K: 'a + fmt::Debug, V: 'a + fmt::Debug> fmt::Debug for HashTrieMapIter<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for HashTrieMapIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        unsafe {
            match self.iter.next() {
                Some(leaf) => Some((&(*leaf.as_ptr()).0, &mut (*leaf.as_ptr()).1)),
                None => None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for HashTrieMapIterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some((&(*leaf.as_ptr()).0, &mut (*leaf.as_ptr()).1)),
                None => None,
            }
        }
    }
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for HashTrieMapIterMut<'a, K, V> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for HashTrieMapIterMut<'a, K, V> {
}

unsafe impl<'a, K: 'a, V: 'a> TrustedLen for HashTrieMapIterMut<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieMapIterMut<'a, K, V> {
    fn clone(&self) -> HashTrieMapIterMut<'a, K, V> {
        HashTrieMapIterMut { iter: self.iter.clone() }
    }
}

impl<'a, K: 'a + fmt::Debug, V: 'a + fmt::Debug> fmt::Debug for HashTrieMapIterMut<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for HashTrieMapKeys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        unsafe {
            match self.iter.next() {
                Some(leaf) => Some(&(*leaf.as_ptr()).0),
                None => None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for HashTrieMapKeys<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a K> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some(&(*leaf.as_ptr()).0),
                None => None,
            }
        }
    }
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for HashTrieMapKeys<'a, K, V> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for HashTrieMapKeys<'a, K, V> {
}

unsafe impl<'a, K: 'a, V: 'a> TrustedLen for HashTrieMapKeys<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieMapKeys<'a, K, V> {
    fn clone(&self) -> HashTrieMapKeys<'a, K, V> {
        HashTrieMapKeys { iter: self.iter.clone() }
    }
}

impl<'a, K: 'a + fmt::Debug, V: 'a> fmt::Debug for HashTrieMapKeys<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for HashTrieMapVals<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<&'a V> {
        unsafe {
            match self.iter.next() {
                Some(leaf) => Some(&(*leaf.as_ptr()).1),
                None => None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for HashTrieMapVals<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a V> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some(&(*leaf.as_ptr()).1),
                None => None,
            }
        }
    }
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for HashTrieMapVals<'a, K, V> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for HashTrieMapVals<'a, K, V> {
}

unsafe impl<'a, K: 'a, V: 'a> TrustedLen for HashTrieMapVals<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieMapVals<'a, K, V> {
    fn clone(&self) -> HashTrieMapVals<'a, K, V> {
        HashTrieMapVals { iter: self.iter.clone() }
    }
}

impl<'a, K: 'a , V: 'a + fmt::Debug> fmt::Debug for HashTrieMapVals<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for HashTrieMapValsMut<'a, K, V> {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<&'a mut V> {
        unsafe {
            match self.iter.next() {
                Some(leaf) => Some(&mut (*leaf.as_ptr()).1),
                None => None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.iter.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for HashTrieMapValsMut<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a mut V> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some(&mut (*leaf.as_ptr()).1),
                None => None,
            }
        }
    }
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for HashTrieMapValsMut<'a, K, V> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for HashTrieMapValsMut<'a, K, V> {
}

unsafe impl<'a, K: 'a, V: 'a> TrustedLen for HashTrieMapValsMut<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieMapValsMut<'a, K, V> {
    fn clone(&self) -> HashTrieMapValsMut<'a, K, V> {
        HashTrieMapValsMut { iter: self.iter.clone() }
    }
}

impl<'a, K: 'a , V: 'a + fmt::Debug> fmt::Debug for HashTrieMapValsMut<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
