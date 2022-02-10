use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::{ExactSizeIterator, FusedIterator, TrustedLen};
use tg_core::murmur3::Murmur3;
use tg_mem::alloc::{Hold, Holder, HoldError, Stow, TryClone, CloneIntoHold};
use crate::hash_trie::{HashTrie, HashTrieIter};

/// Hash array mapped trie set.
pub struct HashTrieSet<'a, T, H = Murmur3> {
    trie: HashTrie<'a, T, (), H>,
}

/// Iterator over the leafs of a `HashTrieSet`.
pub struct HashTrieSetIter<'a, T: 'a> {
    iter: HashTrieIter<'a, T, ()>
}

impl<T> HashTrieSet<'static, T> {
    /// Constructs a new `HashTrieSet` that will allocate its data in the
    /// global `Hold`.
    #[inline]
    pub fn new() -> HashTrieSet<'static, T> {
        HashTrieSet::hold_new(Hold::global())
    }
}

impl<T, H> HashTrieSet<'static, T, H> {
    /// Constructs a new `HashTrieSet` that will allocate its data in the
    /// global `Hold`, and hash its keys using the supplied `hasher`.
    #[inline]
    pub fn new_hasher(hasher: H) -> HashTrieSet<'static, T, H> {
        HashTrieSet::hold_new_hasher(Hold::global(), hasher)
    }
}

impl<'a, T> HashTrieSet<'a, T> {
    /// Constructs a new `HashTrieSet` that will allocate its data in `Hold`.
    /// Allocates a zero-sized root block in `hold`, which typically returns a
    /// shared sentinel pointer to the hold, consuming no additional memory.
    #[inline]
    pub fn hold_new(hold: &dyn Hold<'a>) -> HashTrieSet<'a, T> {
        HashTrieSet { trie: HashTrie::hold_new(hold) }
    }
}

impl<'a, T, H> HashTrieSet<'a, T, H> {
    /// Constructs a new `HashTrieSet` that will allocate its data in `Hold`,
    /// and hash its keys using the supplied `hasher`. Allocates a zero-sized
    /// root block in `hold`, which typically returns a shared sentinel pointer
    /// to the hold, consuming no additional memory.
    #[inline]
    pub fn hold_new_hasher(hold: &dyn Hold<'a>, hasher: H) -> HashTrieSet<'a, T, H> {
        HashTrieSet { trie: HashTrie::hold_new_hasher(hold, hasher) }
    }

    /// Returns `true` if this `HashTrieSet` contains no leafs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trie.is_empty()
    }

    /// Returns the number of leafs in this `HashTrieSet`.
    #[inline]
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns an iterator over the leafs of this `HashTrieSet`.
    pub fn iter(&self) -> HashTrieSetIter<'a, T> {
        HashTrieSetIter { iter: self.trie.iterator() }
    }
}

impl<'a, T: Eq + Hash, H: BuildHasher> HashTrieSet<'a, T, H> {
    /// Returns `true` if this `HashTrieSet` contains the given `elem`.
    pub fn contains<U: Borrow<T> + ?Sized>(&self, elem: &U) -> bool {
        self.trie.contains_key(elem)
    }

    /// Includes a new `elem` in this `HashTrieSet`; returns `true` if the
    /// set already contained `elem`. If the trie's `Hold` fails to allocate
    /// any required new memory, returns the `elem` along with a `HoldError`,
    /// and leaves the trie in its original state.
    pub fn insert(&mut self, elem: T) -> Result<bool, (T, HoldError)> {
        match self.trie.insert(elem, ()) {
            Ok(Some(_)) => Ok(false),
            Ok(None) => Ok(true),
            Err((elem, _, error)) => Err((elem, error)),
        }
    }

    /// Excludes the given `elem` from this `HashTrieSet`; returns `true` if
    /// the set previously contained `elem`. Returns a `HoldError`, and leaves
    /// the trie in its original state, if the trie's `Hold` fails to allocate
    /// any required new memory.
    pub fn remove<U: Borrow<T> + ?Sized>(&mut self, elem: &U) -> Result<bool, HoldError> {
        match self.trie.remove(elem) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl<'a, T, H> Holder<'a> for HashTrieSet<'a, T, H> {
    #[inline]
    fn holder(&self) -> &'a dyn Hold<'a> {
        self.trie.holder()
    }
}

impl<'a, T: Clone, H: Clone> TryClone for HashTrieSet<'a, T, H> {
    fn try_clone(&self) -> Result<HashTrieSet<'a, T, H>, HoldError> {
        Ok(HashTrieSet { trie: self.trie.try_clone()? })
    }
}

impl<'a, T: Clone, H: Clone> CloneIntoHold<'a, HashTrieSet<'a, T, H>> for HashTrieSet<'a, T, H> {
    fn try_clone_into_hold(&self, hold: &Hold<'a>) -> Result<HashTrieSet<'a, T, H>, HoldError> {
        Ok(HashTrieSet { trie: self.trie.try_clone_into_hold(hold)? })
    }
}

impl<'a, 'b, T, H> Stow<'b, HashTrieSet<'b, T, H>> for HashTrieSet<'a, T, H>
    where T: Stow<'b>,
          H: Stow<'b>,
{
    unsafe fn stow(src: *mut HashTrieSet<'a, T, H>, dst: *mut HashTrieSet<'b, T, H>, hold: &Hold<'b>)
        -> Result<(), HoldError>
    {
        HashTrie::stow(&mut (*src).trie, &mut (*dst).trie, hold)?;
        Ok(())
    }

    unsafe fn unstow(src: *mut HashTrieSet<'a, T, H>, dst: *mut HashTrieSet<'b, T, H>) {
        HashTrie::unstow(&mut (*src).trie, &mut (*dst).trie);
    }
}

impl<'a, T, H> IntoIterator for &'a HashTrieSet<'a, T, H> {
    type Item = &'a T;
    type IntoIter = HashTrieSetIter<'a, T>;

    #[inline]
    fn into_iter(self) -> HashTrieSetIter<'a, T> {
        self.iter()
    }
}

impl<'a, T: 'a + fmt::Debug> fmt::Debug for HashTrieSet<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, T: 'a> Iterator for HashTrieSetIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
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

impl<'a, T: 'a> DoubleEndedIterator for HashTrieSetIter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        unsafe {
            match self.iter.next_back() {
                Some(leaf) => Some(&(*leaf.as_ptr()).0),
                None => None,
            }
        }
    }
}

impl<'a, T: 'a> ExactSizeIterator for HashTrieSetIter<'a, T> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.iter.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T: 'a> FusedIterator for HashTrieSetIter<'a, T> {
}

unsafe impl<'a, T: 'a> TrustedLen for HashTrieSetIter<'a, T> {
}

impl<'a, T: 'a> Clone for HashTrieSetIter<'a, T> {
    fn clone(&self) -> HashTrieSetIter<'a, T> {
        HashTrieSetIter { iter: self.iter.clone() }
    }
}

impl<'a, T: 'a + fmt::Debug> fmt::Debug for HashTrieSetIter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
