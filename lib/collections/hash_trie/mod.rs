use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};
use core::slice;
use tg_core::murmur3::Murmur3;
use tg_mem::block::{Block, Layout};
use tg_mem::alloc::{AllocTag, Hold, HoldError, Stow, TryClone, CloneIntoHold};

mod map;
mod set;

pub use self::map::{HashTrieMap, HashTrieMapIter, HashTrieMapIterMut,
                    HashTrieMapKeys, HashTrieMapVals, HashTrieMapValsMut};
pub use self::set::{HashTrieSet, HashTrieSetIter};

/// Bit mask with a single 1 bit whose bit index equals a 5 bit value.
/// For example, the 5 bit value `17` corresponds to the `BranchBit` mask
/// with just bit 17 set (`0b00000000000000010000000000000000`).
pub(crate) type BranchBit = u32;

/// 2 bit trie branch type discriminator.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum BranchType {
    /// Trie terminates at branch.
    Void = 0u32,
    /// Trie has leaf at branch.
    Leaf = 1u32,
    /// Trie has sub-tree at branch.
    Node = 2u32,
    /// Trie has hash collision at branch.
    Knot = 3u32,
}

/// Hash array mapped trie.
pub(crate) struct HashTrie<'a, K, V, H = Murmur3> {
    /// Root trie node; zero-sized allocation when empty.
    root: NonNull<Node<'a, K, V>>,
    /// Number of leafs contained in the trie.
    len: usize,
    /// Hasher used to hash keys.
    hasher: H,
}

/// Hash trie node mapping 5 bits of hash space to sub-trees and embedded leafs.
///
/// 5 bit numbers have 32 distinct values. So a `u32` can hold a 1 bit flag for
/// each distinct value of a 5 bit number. Each `limb_map` bit indicates whether
/// or not the 5 bit hash value equal to the bit's index represents a branch in
/// the hash trie. Each `leaf_map` bit indicates whether or not the 5 bit hash
/// value equal to the bit's index represents a leaf in the hash trie.
///
/// For a given 5 bit hash value, if neither its `limb_map` bit nor its
/// `leaf_map` bit is set, then the trie terminates for that hash value.
/// If both its `limb_map` bit and its `leaf_map` bit are set, then a hash
/// collision exists for that hash value, and the trie branches to a knot.
///
/// Nodes have the following in-memory layout:
///
/// ```text
/// struct Node<'a, K, V> {
///     limb_map: u32,
///     leaf_map: u32,
///     limbs: [Limb<'a, K, V>; limb_map.count_ones()],
///     leafs: [(K, V); (leaf_map & !limb_map).count_ones()],
/// }
/// ```
struct Node<'a, K, V> {
    /// Bit mask of 5 bit hash values that branch to sub-trees.
    /// Each bit represents the 5 bit hash value equal to its bit index.
    limb_map: u32,
    /// Bit mask of 5 bit hash values that branch to embedded leafs.
    /// Each bit represents the 5 bit hash value equal to its bit index.
    leaf_map: u32,
    /// Variant over K and V, with drop check.
    data_marker: PhantomData<[(K, V)]>,
    /// Variant over 'a.
    hold_marker: PhantomData<&'a ()>,
}

/// Hash trie collision bucket.
///
/// Knots have the following in-memory structure:
///
/// ```text
/// struct Knot<'a, K, V> {
///     hash: u64,
///     len: usize,
///     leafs: [(K, V); len],
/// }
/// ```
struct Knot<'a, K, V> {
    /// Hash code shared by all keys in the knot.
    hash: u64,
    /// Number of leafs contained in the knot.
    len: usize,
    /// Variant over K and V, with drop check.
    data_marker: PhantomData<[(K, V)]>,
    /// Variant over 'a.
    hold_marker: PhantomData<&'a ()>,
}

/// Hash trie branch; either a `Node` or a `Knot`. Discriminated by a
/// `BranchType` extracted from a `limb_map` and `leaf_map`.
union Limb<'a, K, V> {
    #[allow(dead_code)]
    node: Node<'a, K, V>,
    #[allow(dead_code)]
    knot: Knot<'a, K, V>,
}

/// Result of inserting a key, value pair into a `Node`.
enum NodeInsert<'a, K, V> {
    /// Leaf inserted into a descendant. The trie has been mutated in place,
    /// so the insert must not subsequently fail.
    None,
    /// Updated an existing leaf in a descendant, replacing the returned value.
    /// The trie has been mutated in place, so the insert must not subsequently
    /// fail.
    Diff(V),
    /// Allocated a copy of the node with the new leaf inserted. The old node
    /// has been left intact in case the insert subsequently fails. Caller must
    /// drop the old node if the insert eventually succeeds.
    Copy(*mut Node<'a, K, V>),
    /// Allocation error occurred; the node has been left intact.
    Fail(HoldError),
}

/// Result of removing a key from a `Node`.
enum NodeRemove<'a, K, V> {
    /// Key not found; node not modified.
    None,
    /// Removed the returned leaf from a descendant. The trie has been mutated
    /// in place, so the remove must not subsequently fail.
    Diff((K, V)),
    /// Removed the returned leaf, which was the only leaf left in the node.
    /// The old node has been left intact in case the remove subsequentlu fails.
    /// Caller must drop the old node if the remove eventually succeeds.
    Drop((K, V)),
    /// Removed the returned leaf, as well as the only remaining leaf left
    /// in the node. The old node has been left intact in case the remove
    /// subsequently fails. Caller must drop the old node if the remove
    /// eventually succeeds.
    Lift((K, V), (K, V)),
    /// Allocated a copy of the node with the returned leaf removed. The old
    /// node has been left intact in case the insert subsequently fails. Caller
    /// must drop the old node if the remove eventually succeeds.
    Copy((K, V), *mut Node<'a, K, V>),
    /// Allocation error occurred; the node has been left intact.
    Fail(HoldError),
}

/// Result of inserting a key, value pair into a `Knot`.
enum KnotInsert<'a, K, V> {
    /// Updated an existing leaf in the knot, replacing the returned value.
    /// The knot has been mutated in place, so the insert must not subsequently
    /// fail.
    Diff(V),
    /// Allocated a copy of the knot with the new leaf inserted. The old knot
    /// has been left intact in case the insert subsequently fails. Caller must
    /// drop the old knot if the insert eventually succeeds.
    Copy(*mut Knot<'a, K, V>),
    /// Allocation error occurred; the knot has been left intact.
    Fail(HoldError),
}

/// Result of removing a key from a `Knot`.
enum KnotRemove<'a, K, V> {
    /// Key not found; knot not modified.
    None,
    /// The removed leaf, which was the only leaf left in the knot node. Old knot
    /// node left intact in case remove subsequently fails. Caller must drop old knot
    /// node if remove fully succeeds.
    Drop((K, V)),
    /// Removed the returned leaf, as well as the only remaining leaf left
    /// in the knot. The old knot has been left intact in case the remove
    /// subsequently fails. Caller must drop the old knot if the remove
    /// eventually succeeds.
    Lift((K, V), (K, V)),
    /// Allocated a copy of the knot with the returned leaf removed. The old
    /// knot has been left intact in case the remove subsequently fails. Caller
    /// must drop the old knot if the remove eventually succeeds.
    Copy((K, V), *mut Knot<'a, K, V>),
    /// Allocation error occurred; the knot has been left intact.
    Fail(HoldError),
}

/// Hash trie iterator stack frame.
enum IterFrame<'a, K, V> {
    /// Terminated.
    Void,
    /// Descended into a node.
    Node {
        limb_map: u32,
        leaf_map: u32,
        branch: BranchBit,
        limb_ptr: *mut *mut Limb<'a, K, V>,
        leaf_ptr: *mut (K, V),
    },
    /// Descended into a collision bucket.
    Knot {
        head_ptr: *mut (K, V),
        foot_ptr: *mut (K, V),
    },
}

/// Hash trie iteration stack.
pub(crate) struct HashTrieIter<'a, K, V> {
    /// Number of leafs remaining to be iterated over.
    count: usize,
    /// Index of the top frame in the iterator stack.
    depth: i8,
    /// Current path through the trie; max depth of 13 nodes for 64 bit hash
    /// codes, plus 1 possible knot.
    stack: [IterFrame<'a, K, V>; 14],
}

/// Computes the hash code of `key` using the supplied `hasher`.
#[inline]
fn hash_key<K, H>(hasher: &H, key: &K) -> u64
    where K: Hash + ?Sized,
          H: BuildHasher,
{
    let mut h = hasher.build_hasher();
    key.hash(&mut h);
    h.finish()
}

/// Returns a bit mask containing a single 1 bit, whose bit index equals the
/// low 5 bits of the `hash` value after shifting it right by `shift` bits.
#[inline]
fn branch32(hash: u64, shift: u32) -> BranchBit {
    1 << ((hash >> shift) as u32 & 0x1F)
}

impl BranchType {
    /// Extracts the 2 bit `BranchType` of the masked bit in `branch` from
    /// the given `limb_map` and `leaf_map`.
    #[inline]
    fn for_branch(limb_map: u32, leaf_map: u32, branch: BranchBit) -> BranchType {
        // Initialize the discriminant to Void.
        let mut discriminant = 0;
        // Check if the branch bit in the limb_map is set.
        if limb_map & branch != 0 {
            // Branch is a node or knot.
            discriminant |= BranchType::Node as u32;
        }
        // Check if the branch bit in the leaf_map is set.
        if leaf_map & branch != 0 {
            // Branch is a leaf or knot.
            discriminant |= BranchType::Leaf as u32;
        }
        // Reinterpret the discriminant as a BranchType.
        unsafe { mem::transmute(discriminant) }
    }
}

impl<'a, K, V> HashTrie<'a, K, V> {
    /// Constructs a new `HashTrie` that will allocate its data in `Hold`.
    /// Allocates a zero-sized root block in `hold`, which typically returns a
    /// shared sentinel pointer to the hold, consuming no additional memory.
    #[inline]
    pub(crate) fn hold_new(hold: &dyn Hold<'a>) -> HashTrie<'a, K, V> {
        unsafe {
            // Construct an empty root node in the hold.
            let root = Node::empty(hold);
            // Initialize the trie.
            HashTrie {
                root: NonNull::new_unchecked(root),
                len: 0,
                hasher: Murmur3::new(),
            }
        }
    }
}

impl<'a, K, V, H> HashTrie<'a, K, V, H> {
    /// Constructs a new `HashTrie` that will allocate its data in `Hold`,
    /// and hash its keys using the supplied `hasher`. Allocates a zero-sized
    /// root block in `hold`, which typically returns a shared sentinel pointer
    /// to the hold, consuming no additional memory.
    #[inline]
    pub(crate) fn hold_new_hasher(hold: &dyn Hold<'a>, hasher: H) -> HashTrie<'a, K, V, H> {
        unsafe {
            // Construct an empty root node in the hold.
            let root = Node::empty(hold);
            // Initialize the trie.
            HashTrie {
                root: NonNull::new_unchecked(root),
                len: 0,
                hasher: hasher,
            }
        }
    }

    /// Returns `true` if this `HashTrie` contains no leafs.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of leafs in this `HashTrie`.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Returns a reference to the `Hold` that allocates this `HashTrie`.
    #[inline]
    pub(crate) fn holder(&self) -> &'a dyn Hold<'a> {
        let root = self.root.as_ptr() as *mut u8;
        AllocTag::from_ptr(root).holder()
    }

    /// Returns a raw iterator over the leafs of this `HashTrie`.
    #[inline]
    pub(crate) fn iterator(&self) -> HashTrieIter<'a, K, V> {
        unsafe {
            if self.len != 0 {
                HashTrieIter::new(self.len, IterFrame::from_node(self.root.as_ptr()))
            } else {
                HashTrieIter::empty()
            }
        }
    }
}

impl<'a, K: Eq + Hash, V, H: BuildHasher> HashTrie<'a, K, V, H> {
    /// Returns `true` if this `HashTrie` contains the given `key`.
    pub(crate) fn contains_key<J: Borrow<K> + ?Sized>(&self, key: &J) -> bool {
        unsafe {
            // Check if the root node exists.
            if self.len != 0 {
                // Hash the lookup key.
                let hash = hash_key(&self.hasher, key.borrow());
                // Search the trie for the key.
                self.root.as_ptr().contains_key(key.borrow(), hash, 0)
            } else {
                // No keys in an empty trie.
                false
            }
        }
    }

    /// Returns the value associated with the given `key`, or `None` if no
    /// association exists.
    pub(crate) fn get<J: Borrow<K> + ?Sized>(&self, key: &J) -> Option<&V> {
        unsafe {
            // Check if the root node exists.
            if self.len != 0 {
                // Hash the lookup key.
                let hash = hash_key(&self.hasher, key.borrow());
                // Search the trie for a value associated with the key.
                self.root.as_ptr().get(key.borrow(), hash, 0)
            } else {
                // No associations in an empty trie.
                None
            }
        }
    }

    /// Associates a new `value` with the given `key`; returns the previous
    /// value associated with the `key`, if defined. If the trie's `Hold` fails
    /// to allocate any required new memory, returns the `key` and `value`,
    /// along with a `HoldError`, and leaves the trie in its original state.
    pub(crate) fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V, HoldError)> {
        unsafe {
            // Hash the insert key.
            let hash = hash_key(&self.hasher, &key);
            // Get a pointer to the root node.
            let old_root = self.root.as_ptr();
            // Get the current length of the trie.
            let old_len = self.len;
            // Check if the root node exists.
            if old_len != 0 {
                // Trie is non-empty; try to insert the new key and value.
                match old_root.insert(&self.hasher, &key, &value, hash, 0) {
                    // Successfully inserted into descendant.
                    NodeInsert::None => {
                        // Increment the length of the trie; can't overflow.
                        self.len = old_len.wrapping_add(1);
                        // No previous value.
                        Ok(None)
                    },
                    // Successfully updated descendant.
                    NodeInsert::Diff(old_val) => {
                        // Return the previous value.
                        Ok(Some(old_val))
                    },
                    // Successfully inserted into a copy of the root node.
                    NodeInsert::Copy(new_node) => {
                        // Deallocate the old root node.
                        old_root.dealloc();
                        // Increment the length of the trie; can't overflow.
                        self.len = old_len.wrapping_add(1);
                        // Update the root node pointer.
                        self.root = NonNull::new_unchecked(new_node);
                        // No previous value.
                        Ok(None)
                    },
                    // Insert failed.
                    NodeInsert::Fail(error) => Err((key, value, error)),
                }
            } else {
                // Trie is empty; allocate a new root node, populated with the
                // new key and value.
                let root = match Node::unary(old_root.holder(), &key, &value, hash) {
                    Ok(root) => root,
                    Err(error) => return Err((key, value, error)),
                };
                // Allocation succeeded; deallocate the old root node.
                old_root.dealloc();
                // Update the root node pointer.
                self.root = NonNull::new_unchecked(root);
                // Set the length of the trie.
                self.len = 1;
                // No previous value associated with the insert key.
                Ok(None)
            }
        }
    }

    /// Disassociates the given `key`; returns the previous value associated
    /// with the `key`, if any. Returns a `HoldError`, and leaves the trie in
    /// its original state, if the trie's `Hold` fails to allocate any required
    /// new memory.
    pub(crate) fn remove<J: Borrow<K> + ?Sized>(&mut self, key: &J) -> Result<Option<V>, HoldError> {
        unsafe {
            // Get the current length of the trie.
            let old_len = self.len;
            // Check if the root node exists.
            if old_len != 0 {
                // Trie is non-empty; hash the remove key.
                let hash = hash_key(&self.hasher, key.borrow());
                // Get a pointer to the root node.
                let old_root = self.root.as_ptr();
                // Try to remove the key.
                match old_root.remove(key.borrow(), hash, 0) {
                    // No association found.
                    NodeRemove::None => Ok(None),
                    // Successfully removed the key from a descendant.
                    NodeRemove::Diff((old_key, old_val)) => {
                        // Drop the old key.
                        mem::drop(old_key);
                        // Decrement the length of the trie.
                        self.len = old_len.wrapping_sub(1);
                        // Return the removed value.
                        Ok(Some(old_val))
                    },
                    // Successfully removed the last key in the trie.
                    NodeRemove::Drop((old_key, old_val)) => {
                        // Crop the old key.
                        mem::drop(old_key);
                        // Construct a new empty root node in the hold.
                        let new_root = Node::<'a, K, V>::empty(old_root.holder());
                        // Deallocate the old root node.
                        old_root.dealloc();
                        // Update the root node pointer.
                        self.root = NonNull::new_unchecked(new_root);
                        // Reset the trie length.
                        self.len = 0;
                        // Return the removed value.
                        Ok(Some(old_val))
                    },
                    // Successfully removed the next-to-last key in the trie.
                    NodeRemove::Lift((old_key, old_val), (new_key, new_val)) => {
                        // Hash the remaining key.
                        let new_hash = hash_key(&self.hasher, &new_key);
                        // Allocate a new root node, populated with the remaining
                        // key and value.
                        let new_root = match Node::unary(old_root.holder(), &new_key, &new_val, new_hash) {
                            Ok(new_root) => new_root,
                            Err(error) => return Err(error),
                        };
                        // Deallocate the old root node.
                        old_root.dealloc();
                        // Drop the old key.
                        mem::drop(old_key);
                        // Forget the new key, which moved to the new root node.
                        mem::forget(new_key);
                        // Forget the new value, which moved to the new root node.
                        mem::forget(new_val);
                        // Update the root node pointer.
                        self.root = NonNull::new_unchecked(new_root);
                        // Set the length of the trie.
                        self.len = 1;
                        // No previous value associated with the key.
                        Ok(Some(old_val))
                    },
                    // Successfully removed from a copy of the root node.
                    NodeRemove::Copy((old_key, old_val), new_node) => {
                        // Deallocate the old root node.
                        old_root.dealloc();
                        // Drop the old key.
                        mem::drop(old_key);
                        // Update the root node pointer.
                        self.root = NonNull::new_unchecked(new_node);
                        // Decrement the length of the trie.
                        self.len = old_len.wrapping_sub(1);
                        // Return the removed value.
                        Ok(Some(old_val))
                    },
                    // Remove failed.
                    NodeRemove::Fail(error) => return Err(error),
                }
            } else {
                // Trie is empty; no associations exists.
                Ok(None)
            }
        }
    }
}

unsafe impl<'a, K: Send, V: Send, H: Send> Send for HashTrie<'a, K, V, H> {
}

unsafe impl<'a, K: Sync, V: Sync, H: Sync> Sync for HashTrie<'a, K, V, H> {
}

unsafe impl<'a, #[may_dangle] K, #[may_dangle] V, H> Drop for HashTrie<'a, K, V, H> {
    fn drop(&mut self) {
        unsafe {
            // Get a pointer to the root node.
            let root = self.root.as_ptr();
            // Check if the root node exists.
            if self.len != 0 {
                // Trie is non-empty; drop the root node.
                root.drop();
            } else {
                // Trie is empty; reconstruct the zero-sized root block.
                let block = Block::from_raw_parts(root as *mut u8, 0);
                // Deallocate the zero-sized root block.
                root.holder().dealloc(block);
            }
        }
    }
}

impl<'a, K: Clone, V: Clone, H: Clone> TryClone for HashTrie<'a, K, V, H> {
    fn try_clone(&self) -> Result<HashTrie<'a, K, V, H>, HoldError> {
        unsafe {
            // Get a pointer to the root node.
            let old_root = self.root.as_ptr();
            // Get the length of the trie.
            let len = self.len;
            // Check if the root node exists.
            if len != 0 {
                // Recursively clone the trie into the new hold, bailing on failure.
                let new_root = old_root.clone_tree(old_root.holder())?;
                // Return the cloned trie.
                Ok(HashTrie {
                    root: NonNull::new_unchecked(new_root),
                    len: len,
                    hasher: self.hasher.clone(),
                })
            } else {
                // Return an empty trie in the new hold.
                Ok(HashTrie::hold_new_hasher(self.holder(), self.hasher.clone()))
            }
        }
    }
}

impl<'a, K: Clone, V: Clone, H: Clone> CloneIntoHold<'a, HashTrie<'a, K, V, H>> for HashTrie<'a, K, V, H> {
    fn try_clone_into_hold(&self, hold: &dyn Hold<'a>) -> Result<HashTrie<'a, K, V, H>, HoldError> {
        unsafe {
            // Get a pointer to the root node.
            let old_root = self.root.as_ptr();
            // Get the length of the trie.
            let len = self.len;
            // Check if the root node exists.
            if len != 0 {
                // Recursively clone the trie into the new hold, bailing on failure.
                let new_root = old_root.clone_tree(hold)?;
                // Return the cloned trie.
                Ok(HashTrie {
                    root: NonNull::new_unchecked(new_root),
                    len: len,
                    hasher: self.hasher.clone(),
                })
            } else {
                // Return an empty trie in the new hold.
                Ok(HashTrie::hold_new_hasher(hold, self.hasher.clone()))
            }
        }
    }
}

impl<'a, 'b, K, V, H> Stow<'b, HashTrie<'b, K, V, H>> for HashTrie<'a, K, V, H>
    where K: Stow<'b>,
          V: Stow<'b>,
          H: Stow<'b>,
{
    unsafe fn stow(src: *mut HashTrie<'a, K, V, H>, dst: *mut HashTrie<'b, K, V, H>, hold: &Hold<'b>)
        -> Result<(), HoldError>
    {
        // Get a pointer to the source root node.
        let old_root = (*src).root.as_ptr();
        // Get the length of the source trie.
        let len = (*src).len;
        // Try to stow the hasher.
        if let err @ Err(_) = H::stow(&mut (*src).hasher, &mut (*dst).hasher, hold) {
            return err;
        }
        // Check if the root node exists.
        if len != 0 {
            // Recursively reallocate the trie in the new hold.
            let new_root = match old_root.move_tree(hold) {
                Ok(new_root) => new_root,
                Err(error) => {
                    // Unstow the moved hasher.
                    H::unstow(&mut (*src).hasher, &mut (*dst).hasher);
                    // Before returning the error.
                    return Err(error);
                },
            };
            // Write the length of the destination trie.
            ptr::write(&mut (*dst).len, ptr::read(&(*src).len));
            // Write the new root node of the destination trie.
            ptr::write(&mut (*dst).root, NonNull::new_unchecked(new_root));
            // Zero the source trie length;
            ptr::write(&mut (*src).len, 0);
            // Zero the root node of the source trie.
            ptr::write(&mut (*src).root, NonNull::new_unchecked(Node::empty(old_root.holder())));
            // Deallocate the old trie, without dropping any leafs.
            old_root.dealloc_tree();
        } else {
            // Write an empty trie in the destination.
            ptr::write(&mut (*dst).len, 0);
            ptr::write(&mut (*dst).root, NonNull::new_unchecked(Node::empty(hold)));
        }
        Ok(())
    }

    unsafe fn unstow(_src: *mut HashTrie<'a, K, V, H>, _dst: *mut HashTrie<'b, K, V, H>) {
        panic!("unsupported");
    }
}

impl<'a, K, V> Node<'a, K, V> {
    /// Constructs a empty `Node` that will allocate its data in `hold`.
    unsafe fn empty(hold: &dyn Hold<'a>) -> *mut Node<'a, K, V> {
        // Get a zero-sized layout.
        let layout = Layout::empty();
        // Allocate a zero-sized block in the hold.
        let block = hold.alloc(layout).unwrap();
        // Return a pointer to the empty block.
        block.as_ptr() as *mut Node<'a, K, V>
    }

    /// Allocates a new `Node` in `hold` with uninitialized storage for the
    /// limbs and leafs masked in `limb_map` and `leaf_map`.
    unsafe fn alloc(hold: &dyn Hold<'a>, limb_map: u32, leaf_map: u32)
        -> Result<*mut Node<'a, K, V>, HoldError>
    {
        // Never allocate empty nodes.
        debug_assert!(leaf_map != 0 || limb_map != 0);
        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the layout of the node.
        let layout = Layout::for_type::<Node<'a, K, V>>()
                            .extended_by_array::<*mut *mut Limb<'a, K, V>>(limb_count)?.0
                            .extended_by_array::<(K, V)>(leaf_count)?.0;
        // Allocate the node, bailing on failure.
        let node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
        // Initialize the node's limb map.
        ptr::write(&mut (*node).limb_map, limb_map);
        // Initialize the node's leaf map.
        ptr::write(&mut (*node).leaf_map, leaf_map);
        // Return a pointer to the new node.
        Ok(node)
    }

    /// Releases the memory owned by this `Node`, without deallocating its
    /// descendants, and without dropping any leafs.
    unsafe fn dealloc(self: *mut Node<'a, K, V>) {
        // Capture this node's limb map.
        let limb_map = (*self).limb_map;
        // Capture this node's leaf map.
        let leaf_map = (*self).leaf_map;
        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the layout of the node.
        let layout = Layout::for_type::<Node<'a, K, V>>()
                            .extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count).0
                            .extended_by_array_unchecked::<(K, V)>(leaf_count).0;
        // Get the block of memory owned by the node.
        let block = Block::from_raw_parts(self as *mut u8, layout.size());
        // Deallocate the block.
        self.holder().dealloc(block);
    }

    /// Releases all memory owned by this sub-tree, without dropping any leafs.
    unsafe fn dealloc_tree(self: *mut Node<'a, K, V>) {
        // Capture this node's limb map.
        let mut limb_map = (*self).limb_map;
        // Capture this node's leaf map.
        let mut leaf_map = (*self).leaf_map;
        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Extend the layout to include the limbs.
        let (layout, limb_offset) = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count);
        // Extend the layout to include the leafs.
        let layout = layout.extended_by_array_unchecked::<(K, V)>(leaf_count).0;

        // Get a pointer to the first limb in the limb array.
        let mut limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Loop over the branches of the node.
        while limb_map | leaf_map != 0 {
            // Determine the type of the current branch.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, 1);
            if branch_type == BranchType::Void {
                // Trie terminates at this branch.
            } else if branch_type == BranchType::Leaf {
                // Trie has a leaf at this branch; don't drop it.
            } else {
                // Trie has a limb at this branch; deallocate it.
                if branch_type == BranchType::Node {
                    // Deallocate the sub-tree.
                    (*(limb_ptr as *mut *mut Node<'a, K, V>)).dealloc_tree();
                } else if branch_type == BranchType::Knot {
                    // Deallocate the sub-knot.
                    (*(limb_ptr as *mut *mut Knot<'a, K, V>)).dealloc();
                }
                // Increment the limb pointer.
                limb_ptr = limb_ptr.wrapping_add(1);
            }
            // Shift the limb map to the next branch.
            limb_map >>= 1;
            // Shift the leaf map to the next branch.
            leaf_map >>= 1;
        }

        // Get the block of memory owned by this node.
        let block = Block::from_raw_parts(self as *mut u8, layout.size());
        // Deallocate the block.
        self.holder().dealloc(block);
    }

    /// Releases the memory owned bu this `Node`, after dropping its
    /// descendants and leafs.
    unsafe fn drop(self: *mut Node<'a, K, V>) {
        // Capture this node's limb map.
        let mut limb_map = (*self).limb_map;
        // Capture this node's leaf map.
        let mut leaf_map = (*self).leaf_map;
        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Extend the layout to include the limbs.
        let (layout, limb_offset) = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count);
        // Extend the layout to include the leafs.
        let (layout, leaf_offset) = layout.extended_by_array_unchecked::<(K, V)>(leaf_count);

        // Get a pointer to the first limb in the limb array.
        let mut limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the first leaf in the leaf array.
        let mut leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        // Loop over the branches of the node.
        while limb_map | leaf_map != 0 {
            // Determine the type of the current branch.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, 1);
            if branch_type == BranchType::Void {
                // Trie terminates at this branch.
            } else if branch_type == BranchType::Leaf {
                // Trie has a leaf at this branch; drop it.
                ptr::drop_in_place(leaf_ptr);
                // Increment the leaf pointer.
                leaf_ptr = leaf_ptr.wrapping_add(1);
            } else {
                // Trie has a limb at this branch.
                if branch_type == BranchType::Node {
                    // Drop the sub-tree.
                    (*(limb_ptr as *mut *mut Node<'a, K, V>)).drop();
                } else if branch_type == BranchType::Knot {
                    // Drop the sub-knot.
                    (*(limb_ptr as *mut *mut Knot<'a, K, V>)).drop();
                }
                // Increment the limb pointer.
                limb_ptr = limb_ptr.wrapping_add(1);
            }
            // Shift the limb map to the next branch.
            limb_map >>= 1;
            // Shift the leaf map to the next branch.
            leaf_map >>= 1;
        }

        // Get the block of memory owned by this node.
        let block = Block::from_raw_parts(self as *mut u8, layout.size());
        // Deallocate the block.
        self.holder().dealloc(block);
    }

    /// Allocates a new `Node` in `hold` containing a single leaf at the branch
    /// for the low 5 bit value of `hash`. Copies `key` and `val` to the new
    /// node on success, logically transferring ownership. Returns a `HoldError`
    /// if allocation fails, leaving ownership of `key` and `val` with the caller.
    unsafe fn unary(hold: &dyn Hold<'a>, key: *const K, val: *const V, hash: u64)
        -> Result<*mut Node<'a, K, V>, HoldError>
    {
        // Use an empty limb map.
        let limb_map = 0;
        // Construct a leaf map with a single leaf for the low 5 bit value of the hash code.
        let leaf_map = branch32(hash, 0);
        // Allocate a new node with a single leaf and no limbs, bailing on failure.
        let node = Node::alloc(hold, limb_map, leaf_map)?;
        // Get the offset of the leaf array.
        let leaf_offset = Layout::for_type::<Node<'a, K, V>>()
                                 .aligned_to_type::<*mut Limb<'a, K, V>>()
                                 .aligned_to_type::<(K, V)>()
                                 .size();
        // Get a pointer to the first leaf in the leaf array.
        let leaf_ptr = (node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        // Copy the key to the new leaf, transferring ownership.
        ptr::copy_nonoverlapping(key, &mut (*leaf_ptr).0, 1);
        // Copy the value to the new leaf, transferring ownership.
        ptr::copy_nonoverlapping(val, &mut (*leaf_ptr).1, 1);
        // Return a pointer to the new node.
        Ok(node)
    }

    /// Returns a reference to the `Hold` that allocated this `Node`.
    #[inline]
    unsafe fn holder(self: *mut Node<'a, K, V>) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(self as *mut u8).holder()
    }

    /// Returns a copy of this node, removing–but not deallocating–any limbs
    /// and leafs no longer present in `new_limb_map` and `new_leaf_map`, and
    /// allocating uninitialized capacity for any new limbs and leafs present
    /// in `new_limb_map` and `new_leaf_map`.
    unsafe fn remap(self: *mut Node<'a, K, V>, hold: &dyn Hold<'a>,
                    mut new_limb_map: u32, mut new_leaf_map: u32)
        -> Result<*mut Node<'a, K, V>, HoldError>
    {
        // Never allocate empty nodes.
        debug_assert!(new_leaf_map != 0 || new_limb_map != 0);
        // Capture this node's limb map;
        let mut old_limb_map = (*self).limb_map;
        // Capture this node's leaf map;
        let mut old_leaf_map = (*self).leaf_map;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();

        // Count the number of limbs in the old node.
        let old_limb_count = old_limb_map.count_ones() as usize;
        // Compute the offset of the old limb array.
        let (old_layout, old_limb_offset) =
            layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(old_limb_count);
        // Compute the offset of the old leaf array.
        let old_leaf_offset = old_layout.padded_to_type::<(K, V)>().size();

        // Count the number of limbs in the new node.
        let new_limb_count = new_limb_map.count_ones() as usize;
        // Count the number of leafs in the new node.
        let new_leaf_count = (!new_limb_map & new_leaf_map).count_ones() as usize;
        // Compute the offset of the new limb array.
        let (new_layout, new_limb_offset) =
            layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(new_limb_count);
        // Compute the offset of the new leaf array.
        let (new_layout, new_leaf_offset) =
            new_layout.extended_by_array_unchecked::<(K, V)>(new_leaf_count);

        // Allocate the new node in hold.
        let new_node = hold.alloc(new_layout)?.as_ptr() as *mut Node<'a, K, V>;
        // Write the new node's limb map.
        ptr::write(&mut (*new_node).limb_map, new_limb_map);
        // Write the new node's leaf map.
        ptr::write(&mut (*new_node).leaf_map, new_leaf_map);

        // Get a pointer to the old node's limb array.
        let mut old_limb_ptr = (self as *mut u8).wrapping_add(old_limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the old node's leaf array.
        let mut old_leaf_ptr = (self as *mut u8).wrapping_add(old_leaf_offset) as *mut (K, V);

        // Get a pointer to the new node's limb array.
        let mut new_limb_ptr = (new_node as *mut u8).wrapping_add(new_limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the new node's leaf array.
        let mut new_leaf_ptr = (new_node as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);

        // Loop over the branches of the new node.
        while new_limb_map | new_leaf_map != 0 {
            if (old_limb_map & new_limb_map) & 1 != 0 {
                // Old and new nodes both have a node or knot at this branch.
                ptr::copy_nonoverlapping(old_limb_ptr, new_limb_ptr, 1);
                old_limb_ptr = old_limb_ptr.wrapping_add(1);
                new_limb_ptr = new_limb_ptr.wrapping_add(1);
            } else if (old_limb_map | new_limb_map) & 1 == 0 && (old_leaf_map & new_leaf_map) & 1 != 0 {
                // Old and new nodes both have a leaf at this branch.
                ptr::copy_nonoverlapping(old_leaf_ptr, new_leaf_ptr, 1);
                old_leaf_ptr = old_leaf_ptr.wrapping_add(1);
                new_leaf_ptr = new_leaf_ptr.wrapping_add(1);
            } else {
                if old_limb_map & 1 != 0 {
                    // Limb removed from old node.
                    old_limb_ptr = old_limb_ptr.wrapping_add(1);
                } else if old_leaf_map & 1 != 0 {
                    // Leaf removed from old node.
                    old_leaf_ptr = old_leaf_ptr.wrapping_add(1);
                }
                if new_limb_map & 1 != 0 {
                    // Limb inserted into new node.
                    new_limb_ptr = new_limb_ptr.wrapping_add(1);
                } else if new_leaf_map & 1 != 0 {
                    // Leaf inserted into new node.
                    new_leaf_ptr = new_leaf_ptr.wrapping_add(1);
                }
            }
            // Shift the old limb map to the next branch.
            old_limb_map >>= 1;
            // Shift the old leaf map to the next branch.
            old_leaf_map >>= 1;
            // Shift the new limb map to the next branch.
            new_limb_map >>= 1;
            // Shift the new leaf map to the next branch.
            new_leaf_map >>= 1;
        }
        // Return a pointer to the new node.
        Ok(new_node)
    }

    /// Recursively reallocates the trie in a new `hold`.
    unsafe fn move_tree<'b>(self: *mut Node<'a, K, V>, hold: &dyn Hold<'b>)
        -> Result<*mut Node<'b, K, V>, HoldError>
        where K: Stow<'b>,
              V: Stow<'b>,
    {
        // Capture this node's limb map;
        let limb_map = (*self).limb_map;
        // Capture this node's leaf map;
        let leaf_map = (*self).leaf_map;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();

        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the offset of the limb array.
        let (layout, limb_offset) = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count);
        // Compute the offset of the leaf array.
        let (layout, leaf_offset) = layout.extended_by_array_unchecked::<(K, V)>(leaf_count);

        // Allocate a new node in the new hold.
        let new_node = hold.alloc(layout)?.as_ptr() as *mut Node<'b, K, V>;
        // Write the new node's limb map.
        ptr::write(&mut (*new_node).limb_map, limb_map);
        // Write the new node's leaf map.
        ptr::write(&mut (*new_node).leaf_map, leaf_map);

        // Get a pointer to the old node's limb array.
        let mut old_limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the old node's leaf array.
        let mut old_leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);

        // Get a pointer to the new node's limb array.
        let mut new_limb_ptr = (new_node as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the new node's leaf array.
        let mut new_leaf_ptr = (new_node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);

        // Start with the first branch bit.
        let mut branch = 1u32;
        // Loop over the branches of the new node.
        while (limb_map | leaf_map) & !branch.wrapping_sub(1) != 0 {
            // Determine the type of this branch.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
            if branch_type == BranchType::Void {
                // Trie terminates at this branch.
            } else if branch_type == BranchType::Leaf {
                // Trie has a leaf at this branch; stow the leaf in the new node.
                if let Err(error) = Stow::stow(old_leaf_ptr, new_leaf_ptr, hold) {
                    // Loop over the already moved branches of the new node.
                    while (limb_map | leaf_map) & branch.wrapping_sub(1) != 0 {
                        // Select the previous branch.
                        branch >>= 1;
                        // Determine the type of the branch.
                        let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
                        if branch_type == BranchType::Void {
                            // Trie terminates at this branch.
                        } else if branch_type == BranchType::Leaf {
                            // Trie has a stowed leaf at this branch.
                            // Rewind the leaf pointers, and unstow the leaf.
                            old_leaf_ptr = old_leaf_ptr.wrapping_sub(1);
                            new_leaf_ptr = new_leaf_ptr.wrapping_sub(1);
                            Stow::unstow(old_leaf_ptr, new_leaf_ptr);
                        } else {
                            // Trie has a moved limb at this branch.
                            // Rewind the limb pointer to the previous limb.
                            new_limb_ptr = new_limb_ptr.wrapping_sub(1);
                            if branch_type == BranchType::Node {
                                // Deallocate the moved sub-tree.
                                (*new_limb_ptr as *mut Node<'a, K, V>).dealloc_tree();
                            } else if branch_type == BranchType::Knot {
                                // Deallocate the moved sub-knot.
                                (*new_limb_ptr as *mut Knot<'a, K, V>).dealloc();
                            }
                        }
                    }
                    // Deallocate the new node.
                    new_node.dealloc();
                    // Return the error;
                    return Err(error);
                }
                old_leaf_ptr = old_leaf_ptr.wrapping_add(1);
                new_leaf_ptr = new_leaf_ptr.wrapping_add(1);
            } else {
                // Trie has a limb at this branch.
                let old_sub_limb = *old_limb_ptr;
                // Move the sub-limb.
                let new_sub_limb = if branch_type == BranchType::Node {
                    let new_sub_node = (old_sub_limb as *mut Node<'a, K, V>).move_tree(hold);
                    mem::transmute::<_, Result<*mut Limb<'a, K, V>, HoldError>>(new_sub_node)
                } else if branch_type == BranchType::Knot {
                    let new_sub_knot = (old_sub_limb as *mut Knot<'a, K, V>).move_tree(hold);
                    mem::transmute::<_, Result<*mut Limb<'a, K, V>, HoldError>>(new_sub_knot)
                } else {
                    unreachable!()
                };
                match new_sub_limb {
                    // Move succeeded.
                    Ok(new_sub_limb) => {
                        // Write a pointer to the moved limb to the new node.
                        ptr::write(new_limb_ptr, new_sub_limb);
                        old_limb_ptr = old_limb_ptr.wrapping_add(1);
                        new_limb_ptr = new_limb_ptr.wrapping_add(1);
                    },
                    // Move failed.
                    Err(error) => {
                        // Loop over the already moved branches of the new node.
                        while (limb_map | leaf_map) & branch.wrapping_sub(1) != 0 {
                            // Select the previous branch.
                            branch >>= 1;
                            // Determine the type of the branch.
                            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
                            if branch_type == BranchType::Void {
                                // Trie terminates at this branch.
                            } else if branch_type == BranchType::Leaf {
                                // Trie has a stowed leaf at this branch.
                                // Rewind the leaf pointers, and unstow the leaf.
                                old_leaf_ptr = old_leaf_ptr.wrapping_sub(1);
                                new_leaf_ptr = new_leaf_ptr.wrapping_sub(1);
                                Stow::unstow(old_leaf_ptr, new_leaf_ptr);
                            } else {
                                // Trie has a moved limb at this branch.
                                // Rewind the limb pointer to the previous limb.
                                new_limb_ptr = new_limb_ptr.wrapping_sub(1);
                                if branch_type == BranchType::Node {
                                    // Deallocate the moved sub-tree.
                                    (*new_limb_ptr as *mut Node<'a, K, V>).dealloc_tree();
                                } else if branch_type == BranchType::Knot {
                                    // Deallocate the moved sub-knot.
                                    (*new_limb_ptr as *mut Knot<'a, K, V>).dealloc();
                                }
                            }
                        }
                        // Deallocate the new node.
                        new_node.dealloc();
                        // Return the error;
                        return Err(error);
                    },
                }
            }
            // Select the next branch.
            branch <<= 1;
        }
        // Return a pointer to the new node.
        Ok(new_node)
    }

    /// Recursively reallocates the trie in a new `hold`.
    unsafe fn clone_tree(self: *mut Node<'a, K, V>, hold: &dyn Hold<'a>)
        -> Result<*mut Node<'a, K, V>, HoldError>
        where K: Clone, V: Clone
    {
        // Capture this node's limb map;
        let limb_map = (*self).limb_map;
        // Capture this node's leaf map;
        let leaf_map = (*self).leaf_map;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();

        // Count the number of limbs in the node.
        let limb_count = limb_map.count_ones() as usize;
        // Count the number of leafs in the node.
        let leaf_count = (!limb_map & leaf_map).count_ones() as usize;
        // Compute the offset of the limb array.
        let (layout, limb_offset) = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count);
        // Compute the offset of the leaf array.
        let (layout, leaf_offset) = layout.extended_by_array_unchecked::<(K, V)>(leaf_count);

        // Allocate a new node in the new hold.
        let new_node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
        // Write the new node's limb map.
        ptr::write(&mut (*new_node).limb_map, limb_map);
        // Write the new node's leaf map.
        ptr::write(&mut (*new_node).leaf_map, leaf_map);

        // Get a pointer to the old node's limb array.
        let mut old_limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the old node's leaf array.
        let mut old_leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);

        // Get a pointer to the new node's limb array.
        let mut new_limb_ptr = (new_node as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        // Get a pointer to the new node's leaf array.
        let mut new_leaf_ptr = (new_node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);

        // Start with the first branch bit.
        let mut branch = 1u32;
        // Loop over the branches of the new node.
        while (limb_map | leaf_map) & !branch.wrapping_sub(1) != 0 {
            // Determine the type of this branch.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
            if branch_type == BranchType::Void {
                // Trie terminates at this branch.
            } else if branch_type == BranchType::Leaf {
                // Trie has a leaf at this branch; clone the leaf into the new node.
                ptr::write(new_leaf_ptr, (*old_leaf_ptr).clone());
                old_leaf_ptr = old_leaf_ptr.wrapping_add(1);
                new_leaf_ptr = new_leaf_ptr.wrapping_add(1);
            } else {
                // Trie has a limb at this branch.
                let old_sub_limb = *old_limb_ptr;
                // Clone the sub-limb.
                let new_sub_limb = if branch_type == BranchType::Node {
                    let new_sub_node = (old_sub_limb as *mut Node<'a, K, V>).clone_tree(hold);
                    mem::transmute::<_, Result<*mut Limb<'a, K, V>, HoldError>>(new_sub_node)
                } else if branch_type == BranchType::Knot {
                    let new_sub_knot = (old_sub_limb as *mut Knot<'a, K, V>).clone_tree(hold);
                    mem::transmute::<_, Result<*mut Limb<'a, K, V>, HoldError>>(new_sub_knot)
                } else {
                    unreachable!()
                };
                match new_sub_limb {
                    // Clone succeeded.
                    Ok(new_sub_limb) => {
                        // Write a pointer to the cloned limb to the new node.
                        ptr::write(new_limb_ptr, new_sub_limb);
                        old_limb_ptr = old_limb_ptr.wrapping_add(1);
                        new_limb_ptr = new_limb_ptr.wrapping_add(1);
                    },
                    // Clone failed.
                    Err(error) => {
                        // Loop over the already cloned branches of the new node.
                        while (limb_map | leaf_map) & branch.wrapping_sub(1) != 0 {
                            // Select the previous branch.
                            branch >>= 1;
                            // Determine the type of the branch.
                            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
                            if branch_type == BranchType::Void {
                                // Trie terminates at this branch.
                            } else if branch_type == BranchType::Leaf {
                                // Trie has a cloned leaf at this branch.
                                // Rewind the leaf pointer to the previous leaf.
                                new_leaf_ptr = new_leaf_ptr.wrapping_sub(1);
                                // Drop the cloned leaf.
                                ptr::drop_in_place(new_leaf_ptr);
                            } else {
                                // Trie has a cloned limb at this branch.
                                // Rewind the limb pointer to the previous limb.
                                new_limb_ptr = new_limb_ptr.wrapping_sub(1);
                                if branch_type == BranchType::Node {
                                    // Drop the cloned sub-tree.
                                    (*new_limb_ptr as *mut Node<'a, K, V>).drop();
                                } else if branch_type == BranchType::Knot {
                                    // Drop the cloned sub-knot.
                                    (*new_limb_ptr as *mut Knot<'a, K, V>).drop();
                                }
                            }
                        }
                        // Deallocate the new node.
                        new_node.dealloc();
                        // Return the error;
                        return Err(error);
                    },
                }
            }
            // Select the next branch.
            branch <<= 1;
        }
        // Return a pointer to the new node.
        Ok(new_node)
    }

    /// Returns a new node, allocated in `hold` containing two leafs.
    unsafe fn merged_leaf(hold: &dyn Hold<'a>, key0: *const K, val0: *const V, hash0: u64,
                          key1: *const K, val1: *const V, hash1: u64, shift: u32)
        -> Result<*mut Node<'a, K, V>, HoldError>
    {
        // Verify there's no hash collision.
        debug_assert!(hash0 != hash1);
        // Get the branch bit for the next 5 bit string of the first hash code.
        let branch0 = branch32(hash0, shift);
        // Get the branch bit for the next 5 bit string of the second hash code.
        let branch1 = branch32(hash1, shift);

        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Declare the new node.
        let node;
        // Check if the leafs share the branch.
        if branch0 == branch1 {
            // Allocate an intermediary node with a single branch.
            // Compute the layout of the header and limb array, getting the offset of the limb array.
            let (layout, limb_offset) = layout.extended_by_array::<*mut *mut Limb<'a, K, V>>(1)?;
            // Compute the layout of the node, with empty leaf array.
            let layout = layout.padded_to_type::<(K, V)>();
            // Allocate the node in the hold.
            node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
            // Write the node's unary limb map.
            ptr::write(&mut (*node).limb_map, branch0 & branch1);
            // Write the node's empty leaf map.
            ptr::write(&mut (*node).leaf_map, 0);

            // Recursively allocate a sub-node containing the two leafs.
            let sub_node = match Node::merged_leaf(hold, key0, val0, hash0,
                                                   key1, val1, hash1, shift.wrapping_add(5)) {
                Ok(sub_node) => sub_node,
                err @ Err(..) => {
                    // Deallocate the new node.
                    node.dealloc();
                    // Before returning the error.
                    return err;
                },
            };

            // Get a pointer to the node in the limb array.
            let sub_node_ptr = (node as *mut u8).wrapping_add(limb_offset) as *mut *mut Node<'a, K, V>;
            // Write the sub-node to the limb array.
            ptr::write(sub_node_ptr, sub_node);
        } else {
            // Allocate a node with two leafs.
            // Compute the layout of the header and empty limb array.
            let layout = layout.padded_to_type::<*mut Limb<'a, K, V>>();
            // Compute the layout of the node, getting the offset of the leaf array.
            let (layout, leaf_offset) = layout.extended_by_array::<(K, V)>(2)?;
            // Allocate the node in the hold.
            node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
            // Write the node's empty limb map.
            ptr::write(&mut (*node).limb_map, 0);
            // Write the node's binary leaf map.
            ptr::write(&mut (*node).leaf_map, branch0 | branch1);

            // Get a pointer to the leaf array.
            let leaf_ptr = (node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
            // Check the order of the two branches.
            if branch0.wrapping_sub(1) & branch1 == 0 {
                // The first leaf precedes the second leaf, in hash order.
                // Write the first key.
                ptr::copy_nonoverlapping(key0, &mut (*leaf_ptr).0, 1);
                // Write the first value.
                ptr::copy_nonoverlapping(val0, &mut (*leaf_ptr).1, 1);
                // Increment the leaf pointer.
                let leaf_ptr = leaf_ptr.wrapping_add(1);
                // Write the second key.
                ptr::copy_nonoverlapping(key1, &mut (*leaf_ptr).0, 1);
                // Write the second value.
                ptr::copy_nonoverlapping(val1, &mut (*leaf_ptr).1, 1);
            } else {
                // The second leaf precedes the first leaf, in hash order.
                // Write the second key.
                ptr::copy_nonoverlapping(key1, &mut (*leaf_ptr).0, 1);
                // Write the second value.
                ptr::copy_nonoverlapping(val1, &mut (*leaf_ptr).1, 1);
                // Increment the leaf pointer.
                let leaf_ptr = leaf_ptr.wrapping_add(1);
                // Write the first key.
                ptr::copy_nonoverlapping(key0, &mut (*leaf_ptr).0, 1);
                // Write the first value.
                ptr::copy_nonoverlapping(val0, &mut (*leaf_ptr).1, 1);
            }
        }
        // Return a pointer to the new node.
        Ok(node)
    }

    /// Returns a new node, allocated in `hold`, containing a knot and a leaf.
    unsafe fn merged_knot(hold: &dyn Hold<'a>, knot0: *mut Knot<'a, K, V>, hash0: u64,
                          key1: *const K, val1: *const V, hash1: u64, shift: u32)
        -> Result<*mut Node<'a, K, V>, HoldError>
    {
        // Verify there's no hash collision.
        debug_assert!(hash0 != hash1);
        // Get the branch bit for the next 5 bit string of the first hash code.
        let branch0 = branch32(hash0, shift);
        // Get the branch bit for the next 5 bit string of the second hash code.
        let branch1 = branch32(hash1, shift);

        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Declare the new node.
        let node;
        // Check if the knot and the leaf share a branch.
        if branch0 == branch1 {
            // Allocate an intermediary node with a single branch.
            // Compute the layout of the header and limb array, getting the offset of the limb array.
            let (layout, limb_offset) = layout.extended_by_array::<*mut *mut Limb<'a, K, V>>(1)?;
            // Compute the layout of the node, with empty leaf array.
            let layout = layout.padded_to_type::<(K, V)>();
            // Allocate the node in the hold.
            node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
            // Write the node's unary limb map.
            ptr::write(&mut (*node).limb_map, branch0 & branch1);
            // Write the node's empty leaf map.
            ptr::write(&mut (*node).leaf_map, 0);

            // Recursively allocate a sub-node containing the knot and the leaf.
            let sub_node = match Node::merged_knot(hold, knot0, hash0,
                                                   key1, val1, hash1, shift.wrapping_add(5)) {
                Ok(sub_node) => sub_node,
                err @ Err(..) => {
                    // Deallocate the new node.
                    node.dealloc();
                    // Before returning the error.
                    return err;
                },
            };

            // Get a pointer to the sub-node in the limb array.
            let sub_node_ptr = (node as *mut u8).wrapping_add(limb_offset) as *mut *mut Node<'a, K, V>;
            // Write the sub-node to the limb array.
            ptr::write(sub_node_ptr, sub_node);
        } else {
            // Allocate a node with a limb and a leaf.
            // Compute the layout of the header and limb array, getting the offset of the limb array.
            let (layout, limb_offset) = layout.extended_by_array::<*mut *mut Limb<'a, K, V>>(1)?;
            // Compute the layout of the node, getting the offset of the leaf array.
            let (layout, leaf_offset) = layout.extended_by_array::<(K, V)>(1)?;
            // Allocate the node in the hold.
            node = hold.alloc(layout)?.as_ptr() as *mut Node<'a, K, V>;
            // Write the node's unary limb map.
            ptr::write(&mut (*node).limb_map, branch0);
            // Write the node's unary leaf map.
            ptr::write(&mut (*node).leaf_map, branch0 | branch1);

            // Get a pointer to the sub-knot in the limb array.
            let sub_knot_ptr = (node as *mut u8).wrapping_add(limb_offset) as *mut *mut Knot<'a, K, V>;
            // Write the sub-knot to the limb array.
            ptr::write(sub_knot_ptr, knot0);

            // Get a pointer to the leaf in the leaf array.
            let leaf_ptr = (node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
            // Write the key to the leaf.
            ptr::copy_nonoverlapping(key1, &mut (*leaf_ptr).0, 1);
            // Write the value to the leaf.
            ptr::copy_nonoverlapping(val1, &mut (*leaf_ptr).1, 1);
        }
        // Return a pointer to the new node.
        Ok(node)
    }
}

impl<'a, K: Eq + Hash, V> Node<'a, K, V> {
    /// Returns `true` if this node contains the given `key`, branching
    /// off the key's `hash` code shifted right by `shift` bits.
    unsafe fn contains_key(mut self: *mut Node<'a, K, V>, key: &K, hash: u64, mut shift: u32) -> bool {
        // Recursively descend the trie.
        loop {
            // Capture this node's limb map.
            let limb_map = (*self).limb_map;
            // Capture this node's leaf map.
            let leaf_map = (*self).leaf_map;
            // Get the branch bit for the next 5 bit string of the hash code.
            let branch = branch32(hash, shift);
            // Determine the type of branch for the bit string.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
            // Check if the trie terminates at this branch.
            if branch_type == BranchType::Void {
                // Key not found.
                return false;
            } else {
                // Branch exists; compute the layout of the node header.
                let layout = Layout::for_type::<Node<'a, K, V>>();
                // Check if the node has a leaf at this branch.
                if branch_type == BranchType::Leaf {
                    // Count the number of limbs in the node.
                    let limb_count = limb_map.count_ones() as usize;
                    // Get the index of the leaf in the leaf array.
                    let leaf_idx = (!limb_map & leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
                    // Get the offset of the leaf in the leaf array.
                    let leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count).0
                                            .extended_by_array_unchecked::<(K, V)>(leaf_idx).0
                                            .size();
                    // Get a pointer to the leaf.
                    let leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
                    // Return whether or not the leaf's key matches the search key.
                    return &(*leaf_ptr).0 == key;
                } else {
                    // Trie has a limb at this branch.
                    // Get the index of the limb in the limb array.
                    let limb_idx = (limb_map & branch.wrapping_sub(1)).count_ones() as usize;
                    // Get the offset of the limb in the limb array.
                    let limb_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_idx).0
                                            .size();
                    // Get a pointer to the limb.
                    let limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
                    // Check the type of limb at this branch.
                    if branch_type == BranchType::Node {
                        // Descend into the sub-tree at this branch.
                        self = *(limb_ptr as *mut *mut Node<'a, K, V>);
                        // Having matched 5 bits of the hash code.
                        shift += 5;
                        // Recurse.
                        continue;
                    } else if branch_type == BranchType::Knot {
                        // Return whether or not the knot at this branch contains the search key.
                        return (*(limb_ptr as *mut *mut Knot<'a, K, V>)).contains_key(key);
                    }
                }
            }
            unreachable!();
        }
    }

    /// Returns the value associated with the given `key`, branching off the
    /// key's `hash` code shifted right by `shift` bits.
    unsafe fn get<'b, 'c>(mut self: *mut Node<'a, K, V>, key: &'b K, hash: u64, mut shift: u32)
        -> Option<&'c V>
    {
        // Recursively descend the trie.
        loop {
            // Capture this node's limb map.
            let limb_map = (*self).limb_map;
            // Capture this node's leaf map.
            let leaf_map = (*self).leaf_map;
            // Get the branch bit for the next 5 bit string of the hash code.
            let branch = branch32(hash, shift);
            // Determine the type of branch for the bit string.
            let branch_type = BranchType::for_branch(limb_map, leaf_map, branch);
            // Check if the trie terminates at this branch.
            if branch_type == BranchType::Void {
                // Key not found.
                return None;
            } else {
                // Branch exists; compute the layout of the node header.
                let layout = Layout::for_type::<Node<'a, K, V>>();
                // Check if the node has a leaf at this branch.
                if branch_type == BranchType::Leaf {
                    // Count the number of limbs in the node.
                    let limb_count = limb_map.count_ones() as usize;
                    // Get the index of the leaf in the leaf array.
                    let leaf_idx = (!limb_map & leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
                    // Get the offset of the leaf in the leaf array.
                    let leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count).0
                                            .extended_by_array_unchecked::<(K, V)>(leaf_idx).0
                                            .size();
                    // Get a pointer to the leaf.
                    let leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
                    // Check if the leaf key matches the search key.
                    if &(*leaf_ptr).0 == key {
                        // Return a reference to the value of the matched leaf.
                        return Some(&(*leaf_ptr).1);
                    } else {
                        // Keys don't match.
                        return None;
                    }
                } else {
                    // Trie has a limb at this branch.
                    // Get the index of the limb in the limb array.
                    let limb_idx = (limb_map & branch.wrapping_sub(1)).count_ones() as usize;
                    // Get the offset of the limb in the limb array.
                    let limb_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_idx).0
                                            .size();
                    // Get a pointer to the limb.
                    let limb_ptr = (self as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
                    // Check the type of limb at this branch.
                    if branch_type == BranchType::Node {
                        // Descend into the sub-tree at this branch.
                        self = *(limb_ptr as *mut *mut Node<'a, K, V>);
                        // Having matched 5 bits of the hash code.
                        shift += 5;
                        // Recurse.
                        continue;
                    } else if branch_type == BranchType::Knot {
                        // Return the value associated with the search key in the knot.
                        return (*(limb_ptr as *mut *mut Knot<'a, K, V>)).get(key);
                    }
                }
            }
            unreachable!();
        }
    }

    /// Associates a new value with the given key, branching off the key's hash
    /// code shifted right by `shift` bits.
    unsafe fn insert<H: BuildHasher>(self: *mut Node<'a, K, V>, hasher: &H,
                                     new_key: *const K, new_val: *const V, new_hash: u64,
                                     shift: u32)
        -> NodeInsert<'a, K, V>
    {
        // Capture this node's limb map.
        let old_limb_map = (*self).limb_map;
        // Capture this node's leaf map.
        let old_leaf_map = (*self).leaf_map;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Get the branch bit for the next 5 bit string of the hash code.
        let branch = branch32(new_hash, shift);
        // Determine the type of branch for the bit string.
        let branch_type = BranchType::for_branch(old_limb_map, old_leaf_map, branch);
        // Check if the trie terminates at this branch.
        if branch_type == BranchType::Void {
            // No change to the limbs.
            let new_limb_map = old_limb_map;
            // Set leaf flag for branch.
            let new_leaf_map = old_leaf_map | branch;
            // Reallocate the node with a leaf for the branch, bailing on failure.
            let new_node = match self.remap(self.holder(), old_limb_map, new_leaf_map) {
                Ok(new_node) => new_node,
                Err(error) => return NodeInsert::Fail(error),
            };
            // Count the number of limbs in the new node.
            let new_limb_count = new_limb_map.count_ones() as usize;
            // Get the index of the leaf in the new leaf array.
            let new_leaf_idx = (!new_limb_map & new_leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
            // Get the offset of the leaf in the new leaf array.
            let new_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(new_limb_count).0
                                        .extended_by_array_unchecked::<(K, V)>(new_leaf_idx).0
                                        .size();
            // Get a pointer to the new leaf.
            let new_leaf_ptr = (new_node as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
            // Copy the inserted key into the new leaf.
            ptr::copy_nonoverlapping(new_key, &mut (*new_leaf_ptr).0, 1);
            // Copy the inserted value into the new leaf.
            ptr::copy_nonoverlapping(new_val, &mut (*new_leaf_ptr).1, 1);
            // Return a pointer to the new node.
            return NodeInsert::Copy(new_node);
        } else if branch_type == BranchType::Leaf {
            // Trie has a leaf at this branch.
            // Count the number of limbs in the old node.
            let old_limb_count = old_limb_map.count_ones() as usize;
            // Get the index of the leaf in the old leaf array.
            let old_leaf_idx = (!old_limb_map & old_leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
            // Get the offset of the leaf in the old leaf array.
            let old_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(old_limb_count).0
                                        .extended_by_array_unchecked::<(K, V)>(old_leaf_idx).0
                                        .size();
            // Get a pointer to the old leaf.
            let old_leaf_ptr = (self as *mut u8).wrapping_add(old_leaf_offset) as *mut (K, V);
            // Check if the old key matches the new key.
            if &(*old_leaf_ptr).0 == &*new_key {
                // Keys match.
                // Drop the old key.
                ptr::drop_in_place(&mut (*old_leaf_ptr).0);
                // Read out the old value.
                let old_val = ptr::read(&(*old_leaf_ptr).1);
                // Overwrite the old key with the new key.
                ptr::copy_nonoverlapping(new_key, &mut (*old_leaf_ptr).0, 1);
                // Overwrite the old value with the new value.
                ptr::copy_nonoverlapping(new_val, &mut (*old_leaf_ptr).1, 1);
                // Return the old value.
                return NodeInsert::Diff(old_val);
            } else {
                // Keys differ.
                // Set the limb flag for the branch.
                let new_limb_map = old_limb_map | branch;
                // Get the index of the sub-limb in the new limb array.
                let sub_limb_idx = (new_limb_map & branch.wrapping_sub(1)).count_ones() as usize;
                // Get the offset of the sub-limb in the new limb array.
                let sub_limb_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(sub_limb_idx).0
                                            .size();
                // Hash the old key.
                let old_hash = hash_key(hasher, &(*old_leaf_ptr).0);
                // Check if the old hash code differs from the new hash code.
                if old_hash != new_hash {
                    // Hashes differ; merge the leafs into sub-tree.
                    // Unset the leaf flag for the branch.
                    let new_leaf_map = old_leaf_map ^ branch;
                    // Reallocate the node with a limb for the branch, bailing on failure.
                    let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                        Ok(new_node) => new_node,
                        Err(error) => return NodeInsert::Fail(error),
                    };
                    // Allocate a new sub-tree node containing the two leafs.
                    let sub_node = match Node::merged_leaf(self.holder(),
                                                           &(*old_leaf_ptr).0, &(*old_leaf_ptr).1, old_hash,
                                                           new_key, new_val, new_hash,
                                                           shift.wrapping_add(5)) {
                        Ok(sub_node) => sub_node,
                        Err(error) => {
                            // Deallocate the remapped node.
                            new_node.dealloc();
                            // Before returning the error.
                            return NodeInsert::Fail(error);
                        },
                    };
                    // Get a pointer to the sub-node.
                    let sub_node_ptr = (new_node as *mut u8).wrapping_add(sub_limb_offset) as *mut *mut Node<'a, K, V>;
                    // Write the sub-node pointer to the new node.
                    ptr::write(sub_node_ptr, sub_node);
                    // Return a pointer to the new node.
                    return NodeInsert::Copy(new_node);
                } else {
                    // Hashes match; merge the leafs into a sub-knot.
                    // Keep the leaf flag set for the branch.
                    let new_leaf_map = old_leaf_map;
                    // Reallocate the node with a limb for the branch, bailing on failure.
                    let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                        Ok(new_node) => new_node,
                        Err(error) => return NodeInsert::Fail(error),
                    };
                    // Allocate a collision bucket containing the two leafs.
                    let sub_knot = match Knot::binary(self.holder(), new_hash,
                                                      &(*old_leaf_ptr).0, &(*old_leaf_ptr).1,
                                                      new_key, new_val) {
                        Ok(sub_knot) => sub_knot,
                        Err(error) => {
                            // Deallocate the remaped node.
                            new_node.dealloc();
                            // Before returning the error.
                            return NodeInsert::Fail(error);
                        },
                    };
                    // Get a pointer to the sub-knot.
                    let sub_knot_ptr = (new_node as *mut u8).wrapping_add(sub_limb_offset) as *mut *mut Knot<'a, K, V>;
                    // Write the sub-knot pointer to the new node.
                    ptr::write(sub_knot_ptr, sub_knot);
                    // Return a pointer to the new node.
                    return NodeInsert::Copy(new_node);
                }
            }
        } else {
            // Trie has a limb at this branch.
            // Get the index of the sub-limb in the old limb array.
            let sub_limb_idx = (old_limb_map & branch.wrapping_sub(1)).count_ones() as usize;
            // Get the offset of the sub-limb in the old limb array.
            let sub_limb_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(sub_limb_idx).0
                                        .size();
            // Get a pointer to the sub-limb.
            let sub_limb_ptr = (self as *mut u8).wrapping_add(sub_limb_offset) as *mut *mut Limb<'a, K, V>;
            // Check the type of limb at this branch.
            if branch_type == BranchType::Node {
                // Trie has a node at this branch.
                // Get a pointer to the old sub-node.
                let old_sub_node = *(sub_limb_ptr as *mut *mut Node<'a, K, V>);
                // Insert the new key and value into the sub-node.
                match old_sub_node.insert(hasher, new_key, new_val, new_hash, shift.wrapping_add(5)) {
                    // Inserted into a descendant of the sub-node.
                    none @ NodeInsert::None => return none,
                    // Mutated the sub-node in place.
                    diff @ NodeInsert::Diff(..) => return diff,
                    // Inserted into a copy of the sub-node.
                    NodeInsert::Copy(new_sub_node) => {
                        // Deallocate the old sub-node.
                        old_sub_node.dealloc();
                        // Write the new sub-node to the node.
                        ptr::write(sub_limb_ptr as *mut *mut Node<'a, K, V>, new_sub_node);
                        // No previous value.
                        return NodeInsert::None;
                    },
                    // Insert failed.
                    fail @ NodeInsert::Fail(..) => return fail,
                }
            } else if branch_type == BranchType::Knot {
                // Trie has a knot at this branch.
                // Get a pointer to the old sub-knot.
                let old_sub_knot = *(sub_limb_ptr as *mut *mut Knot<'a, K, V>);
                // Get the hash code of the sub-knot.
                let old_hash = (*old_sub_knot).hash;
                // Compare the old hash code to the new hash code.
                if old_hash == new_hash {
                    // Hashes match; insert the new key and value into the knot.
                    match old_sub_knot.insert(new_key, new_val) {
                        // Mutated the sub-knot in place.
                        KnotInsert::Diff(old_val) => return NodeInsert::Diff(old_val),
                        // Inserted into a copy of the sub-knot.
                        KnotInsert::Copy(new_sub_knot) => {
                            // Deallocate the old sub knot.
                            old_sub_knot.dealloc();
                            // Write the new sub-knot to the node.
                            ptr::write(sub_limb_ptr as *mut *mut Knot<'a, K, V>, new_sub_knot);
                            // No previous value.
                            return NodeInsert::None;
                        },
                        // Insert failed.
                        KnotInsert::Fail(error) => return NodeInsert::Fail(error),
                    }
                } else {
                    // Hashes differ; merge the knot and leaf into a new sub-node.
                    let sub_node = match Node::merged_knot(self.holder(), old_sub_knot, old_hash,
                                                           new_key, new_val, new_hash,
                                                           shift.wrapping_add(5)) {
                        Ok(sub_node) => sub_node,
                        Err(error) => return NodeInsert::Fail(error),
                    };
                    // Unset the leaf flag for the branch.
                    (*self).leaf_map = old_leaf_map ^ branch;
                    // Overwrite the old sub-knot with the new sub-node.
                    ptr::write(sub_limb_ptr as *mut *mut Node<'a, K, V>, sub_node);
                    // No previous value.
                    return NodeInsert::None;
                }
            }
        }
        unreachable!();
    }

    /// Removes any association with the given key, branching off the key's
    /// hash code shifted right by `shift` bits.
    unsafe fn remove(self: *mut Node<'a, K, V>, new_key: &K, new_hash: u64, shift: u32)
        -> NodeRemove<'a, K, V>
    {
        // Capture this node's limb map.
        let old_limb_map = (*self).limb_map;
        // Capture this node's leaf map.
        let old_leaf_map = (*self).leaf_map;
        // Compute the layout of the node header.
        let layout = Layout::for_type::<Node<'a, K, V>>();
        // Get the branch bit for the next 5 bit string of the hash code.
        let branch = branch32(new_hash, shift);
        // Determine the type of branch for the bit string.
        let branch_type = BranchType::for_branch(old_limb_map, old_leaf_map, branch);
        // Check if the trie terminates at this branch.
        if branch_type == BranchType::Void {
            // Key not found.
            return NodeRemove::None;
        } else if branch_type == BranchType::Leaf {
            // Trie has a leaf at this branch.
            // Count the number of limbs in the old node.
            let old_limb_count = old_limb_map.count_ones() as usize;
            // Get the index of the leaf in the old leaf array.
            let old_leaf_idx = (!old_limb_map & old_leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
            // Get the layout of the node header and limb array.
            let layout = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(old_limb_count).0;
            // Get the offset of the leaf in the old leaf array.
            let old_leaf_offset = layout.extended_by_array_unchecked::<(K, V)>(old_leaf_idx).0
                                        .size();
            // Get a pointer to the old leaf.
            let old_leaf_ptr = (self as *mut u8).wrapping_add(old_leaf_offset) as *mut (K, V);
            // Check if the old key matches the new key.
            if &(*old_leaf_ptr).0 == new_key {
                // Keys match; read out the old leaf.
                let old_leaf = ptr::read(old_leaf_ptr);
                // No change to the limbs.
                let new_limb_map = old_limb_map;
                // Unset the leaf flag for the branch.
                let new_leaf_map = old_leaf_map ^ branch;
                // Count the number of limbs in the new node.
                let new_limb_count = old_limb_count;
                // Check if the new node has no limbs.
                if new_limb_count == 0 {
                    // Count the number of leafs in the new node.
                    let new_leaf_count = (!new_limb_map & new_leaf_map).count_ones();
                    // Check if the new node is empty.
                    if new_leaf_count == 0 {
                        // Return the removed leaf.
                        return NodeRemove::Drop(old_leaf);
                    }
                    // Check if the new node has a single remaining leaf.
                    if new_leaf_count == 1 {
                        // Get the index of the remaining leaf in the old leaf array.
                        let new_leaf_idx = 1usize.wrapping_sub(old_leaf_idx);
                        // Get the offset of the remaining leaf in the old leaf array.
                        let new_leaf_offset = layout.extended_by_array_unchecked::<(K, V)>(new_leaf_idx).0
                                                    .size();
                        // Get a pointer to the remaining leaf.
                        let new_leaf_ptr = (self as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
                        // Read out the remaining leaf.
                        let new_leaf = ptr::read(new_leaf_ptr);
                        // Return the removed leaf, and the reamining leaf.
                        return NodeRemove::Lift(old_leaf, new_leaf);
                    }
                }
                // Reallocate the node with the leaf removed.
                let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                    Ok(new_node) => new_node,
                    Err(error) => return NodeRemove::Fail(error),
                };
                // Return the removed leaf, and a pointer to the new node.
                return NodeRemove::Copy(old_leaf, new_node);
            } else {
                // Keys differ.
                return NodeRemove::None;
            }
        } else {
            // Trie has a limb at this branch.
            // Get the index of the sub-limb in the old limb array.
            let sub_limb_idx = (old_limb_map & branch.wrapping_sub(1)).count_ones() as usize;
            // Get the offset of the sub-limb in the old limb array.
            let sub_limb_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(sub_limb_idx).0
                                        .size();
            // Get a pointer to the sub-limb.
            let sub_limb_ptr = (self as *mut u8).wrapping_add(sub_limb_offset) as *mut *mut Limb<'a, K, V>;
            // Check the type of limb at this branch.
            if branch_type == BranchType::Node {
                // Trie has a node at this branch.
                // Get a pointer to the old sub-node.
                let old_sub_node = *(sub_limb_ptr as *mut *mut Node<'a, K, V>);
                // Remove the key from the sub-node.
                match old_sub_node.remove(new_key, new_hash, shift.wrapping_add(5)) {
                    // Key not found.
                    none @ NodeRemove::None => return none,
                    // Removed from a descendant of the sub-node.
                    diff @ NodeRemove::Diff(..) => return diff,
                    // Removed the last leaf from the sub-tree.
                    NodeRemove::Drop(old_leaf) => {
                        // Unset the limb flag for the branch.
                        let new_limb_map = old_limb_map ^ branch;
                        // No change to the leafs.
                        let new_leaf_map = old_leaf_map;
                        // Count the number of limbs in the new node.
                        let new_limb_count = new_limb_map.count_ones();
                        // Check if the new node has no more limbs.
                        if new_limb_count == 0 {
                            // Count the number of leafs in the new node.
                            let new_leaf_count = (!new_limb_map & new_leaf_map).count_ones();
                            // Check if the new node is empty.
                            if new_leaf_count == 0 {
                                // Deallocate the old sub-node.
                                old_sub_node.dealloc();
                                // Return the removed leaf.
                                return NodeRemove::Drop(old_leaf);
                            }
                            // Check if the new node has a single remaining leaf.
                            if new_leaf_count == 1 {
                                // Deallocate the old sub-node.
                                old_sub_node.dealloc();
                                // Get the offset of the remaining leaf in the old leaf array.
                                let new_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(1).0
                                                            .padded_to_type::<(K, V)>()
                                                            .size();
                                // Get a pointer to the remaining leaf.
                                let new_leaf_ptr = (self as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
                                // Read out the remaining leaf.
                                let new_leaf = ptr::read(new_leaf_ptr);
                                // Return the removed leaf, and the remaining leaf.
                                return NodeRemove::Lift(old_leaf, new_leaf);
                            }
                        }
                        // New node still has limbs; reallocate the node with the sub-node removed.
                        let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                            Ok(new_node) => new_node,
                            Err(error) => return NodeRemove::Fail(error),
                        };
                        // All allocations succeeded; deallocate the old sub-node.
                        old_sub_node.dealloc();
                        // Return the removed leaf, and a pointer to the new node.
                        return NodeRemove::Copy(old_leaf, new_node);
                    },
                    // Removed the next-to-last leaf from the sub-tree.
                    NodeRemove::Lift(old_leaf, new_leaf) => {
                        // Unset the limb flag for the branch.
                        let new_limb_map = old_limb_map ^ branch;
                        // Set the leaf flag for the branch.
                        let new_leaf_map = old_leaf_map | branch;
                        // Reallocate the node with a leaf, instead of a limb, for the branch.
                        let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                            Ok(new_node) => new_node,
                            Err(error) => return NodeRemove::Fail(error),
                        };
                        // All allocations succeeded; deallocate the old sub-node.
                        old_sub_node.dealloc();
                        // Count the number of limbs in the new node.
                        let new_limb_count = new_limb_map.count_ones() as usize;
                        // Get the index of the leaf in the new leaf array.
                        let new_leaf_idx = (!new_limb_map & new_leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
                        // Get the offset of the leaf in the new leaf array.
                        let new_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(new_limb_count).0
                                                    .extended_by_array_unchecked::<(K, V)>(new_leaf_idx).0
                                                    .size();
                        // Get a pointer to the new leaf.
                        let new_leaf_ptr = (new_node as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
                        // Write the new leaf to the new node.
                        ptr::write(new_leaf_ptr, new_leaf);
                        // Return the removed leaf, and a pointer to the new node.
                        return NodeRemove::Copy(old_leaf, new_node)
                    },
                    // Removed from a copy of the sub-node.
                    NodeRemove::Copy(old_leaf, new_node) => {
                        // Deallocate the old sub-node.
                        old_sub_node.dealloc();
                        // Overwrite the old sub-node pointer with the new sub-node pointer.
                        ptr::write(sub_limb_ptr as *mut *mut Node<'a, K, V>, new_node);
                        // Return the removed leaf.
                        return NodeRemove::Diff(old_leaf)
                    },
                    // Remove failed.
                    fail @ NodeRemove::Fail(..) => return fail,
                }
            } else if branch_type == BranchType::Knot {
                // Trie has a knot at this branch.
                // Get a pointer to the old sub-knot.
                let old_sub_knot = *(sub_limb_ptr as *mut *mut Knot<'a, K, V>);
                // Remove the key from the sub-knot.
                match old_sub_knot.remove(new_key) {
                    // Key not found.
                    KnotRemove::None => return NodeRemove::None,
                    // Removed the last leaf from the sub-knot.
                    KnotRemove::Drop(old_leaf) => {
                        // Unset the limb flag for the branch.
                        let new_limb_map = old_limb_map ^ branch;
                        // No change to the leafs.
                        let new_leaf_map = old_leaf_map;
                        // Count the number of limbs in the new node.
                        let new_limb_count = new_limb_map.count_ones();
                        // Check if the new node has no more limbs.
                        if new_limb_count == 0 {
                            // Count the number of leafs in the new node.
                            let new_leaf_count = (!new_limb_map & new_leaf_map).count_ones();
                            // Check if the new node is empty.
                            if new_leaf_count == 0 {
                                // Deallocate the old sub-knot.
                                old_sub_knot.dealloc();
                                // Return the removed leaf.
                                return NodeRemove::Drop(old_leaf);
                            }
                            // Check if the new node has a single remaining leaf.
                            if new_leaf_count == 1 {
                                // Deallocate the old sub-knot.
                                old_sub_knot.dealloc();
                                // Get the offset of the remaining leaf in the old leaf array.
                                let new_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(1).0
                                                            .padded_to_type::<(K, V)>()
                                                            .size();
                                // Get a pointer to the remaining leaf.
                                let new_leaf_ptr = (self as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
                                // Read out the remaining leaf.
                                let new_leaf = ptr::read(new_leaf_ptr);
                                // Return the removed leaf, and the remaining leaf.
                                return NodeRemove::Lift(old_leaf, new_leaf);
                            }
                        }
                        // New node still has limbs; reallocate the node with the sub-knot removed.
                        let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                            Ok(new_node) => new_node,
                            Err(error) => return NodeRemove::Fail(error),
                        };
                        // All allocations succeeded; deallocate the old sub-knot.
                        old_sub_knot.dealloc();
                        // Return the removed leaf, and a pointer to the new node.
                        return NodeRemove::Copy(old_leaf, new_node);
                    },
                    // Removed the next-to-last leaf from the sub-knot.
                    KnotRemove::Lift(old_leaf, new_leaf) => {
                        // Unset the limb flag for the branch.
                        let new_limb_map = old_limb_map ^ branch;
                        // Set the leaf flag for the branch.
                        let new_leaf_map = old_leaf_map | branch;
                        // Reallocate the node with a leaf, instead of a limb, for the branch.
                        let new_node = match self.remap(self.holder(), new_limb_map, new_leaf_map) {
                            Ok(new_node) => new_node,
                            Err(error) => return NodeRemove::Fail(error),
                        };
                        // All allocations succeeded; deallocate the old sub-knot.
                        old_sub_knot.dealloc();
                        // Count the number of limbs in the new node.
                        let new_limb_count = new_limb_map.count_ones() as usize;
                        // Get the index of the leaf in the new leaf array.
                        let new_leaf_idx = (!new_limb_map & new_leaf_map & branch.wrapping_sub(1)).count_ones() as usize;
                        // Get the offset of the leaf in the new leaf array.
                        let new_leaf_offset = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(new_limb_count).0
                                                    .extended_by_array_unchecked::<(K, V)>(new_leaf_idx).0
                                                    .size();
                        // Get a pointer to the new leaf.
                        let new_leaf_ptr = (new_node as *mut u8).wrapping_add(new_leaf_offset) as *mut (K, V);
                        // Write the new leaf to the new node.
                        ptr::write(new_leaf_ptr, new_leaf);
                        // Return the removed leaf, and a pointer to the new node.
                        return NodeRemove::Copy(old_leaf, new_node);
                    },
                    // Removed from a copy of the sub-knot.
                    KnotRemove::Copy(old_leaf, new_knot) => {
                        // Deallocate the old sub-knot.
                        old_sub_knot.dealloc();
                        // Overwrite the old sub-knot pointer with the new sub-knot pointer.
                        ptr::write(sub_limb_ptr as *mut *mut Knot<'a, K, V>, new_knot);
                        // Return the removed leaf.
                        return NodeRemove::Diff(old_leaf);
                    },
                    // Remove failed.
                    KnotRemove::Fail(error) => return NodeRemove::Fail(error),
                }
            }
        }
        unreachable!();
    }
}

impl<'a, K, V> Knot<'a, K, V> {
    /// Allocate a new `Knot` in `hold` with uninitialized storage for `len` leafs.
    unsafe fn alloc(hold: &dyn Hold<'a>, hash: u64, len: usize) -> Result<*mut Knot<'a, K, V>, HoldError> {
        // Never allocate empty knots.
        debug_assert!(len != 0);
        // Compute the layout of the new knot.
        let layout = Layout::for_type::<Knot<'a, K, V>>().extended_by_array::<(K, V)>(len)?.0;
        // Allocate the new knot, bailing on failure.
        let knot = hold.alloc(layout)?.as_ptr() as *mut Knot<'a, K, V>;
        // Write the hash code of the knot.
        ptr::write(&mut (*knot).hash, hash);
        // Write the length of the knot.
        ptr::write(&mut (*knot).len, len);
        Ok(knot)
    }

    /// Releases the memory owned by this `Knot`, without dropping its leafs.
    unsafe fn dealloc(self: *mut Knot<'a, K, V>) {
        // Capture the length of the knot.
        let len = (*self).len;
        // Compute the layout of the knot.
        let layout = Layout::for_type::<Knot<'a, K, V>>().extended_by_array_unchecked::<(K, V)>(len).0;
        // Get the block of memory owned by the knot.
        let block = Block::from_raw_parts(self as *mut u8, layout.size());
        // Deallocate the block.
        self.holder().dealloc(block);
    }

    /// Releases the memory owned bu this `Knot`, after dropping its leafs.
    unsafe fn drop(self: *mut Knot<'a, K, V>) {
        // Capture the length of the knot.
        let len = (*self).len;
        // Compute the layout of the knot, getting the offset of the leaf array.
        let (layout, leaf_offset) = Layout::for_type::<Knot<'a, K, V>>()
                                                     .extended_by_array_unchecked::<(K, V)>(len);
        // Get a pointer to the leaf array.
        let leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        // Drop the leaf array.
        ptr::drop_in_place(slice::from_raw_parts_mut(leaf_ptr, len));
        // Get the block of memory owned by the knot.
        let block = Block::from_raw_parts(self as *mut u8, layout.size());
        // Deallocate the block.
        self.holder().dealloc(block);
    }

    /// Allocates a new `Knot` in `hold` containing two leafs.
    unsafe fn binary(hold: &dyn Hold<'a>, hash: u64, key0: *const K, value0: *const V,
                     key1: *const K, value1: *const V) -> Result<*mut Knot<'a, K, V>, HoldError> {
        // Allocate a knot with uninitialized capacity for two leafs, bailing on failure.
        let knot = Knot::alloc(hold, hash, 2)?;
        // Get a pointer to the first leaf.
        let leaf_ptr = knot.leaf_array();
        ptr::copy_nonoverlapping(key0, &mut (*leaf_ptr).0, 1);
        ptr::copy_nonoverlapping(value0, &mut (*leaf_ptr).1, 1);
        // Get a pointer to the second leaf.
        let leaf_ptr = leaf_ptr.wrapping_add(1);
        ptr::copy_nonoverlapping(key1, &mut (*leaf_ptr).0, 1);
        ptr::copy_nonoverlapping(value1, &mut (*leaf_ptr).1, 1);
        // Return a pointer to the new knot.
        Ok(knot)
    }

    /// Returns a reference to the `Hold` that allocated this `Knot`.
    #[inline]
    unsafe fn holder(self: *mut Knot<'a, K, V>) -> &'a dyn Hold<'a> {
        AllocTag::from_ptr(self as *mut u8).holder()
    }

    #[inline]
    unsafe fn leaf_array(self: *mut Knot<'a, K, V>) -> *mut (K, V) {
        let offset = Layout::for_type::<Knot<'a, K, V>>().padded_to_type::<(K, V)>().size();
        (self as *mut u8).wrapping_add(offset) as *mut (K, V)
    }

    /// Reallocates this `Knot` in a new `hold`.
    unsafe fn move_tree<'b>(self: *mut Knot<'a, K, V>, hold: &dyn Hold<'b>)
        -> Result<*mut Knot<'b, K, V>, HoldError>
        where K: Stow<'b>,
              V: Stow<'b>,
    {
        let len = (*self).len;
        debug_assert!(len != 0);
        let (layout, leaf_offset) = Layout::for_type::<Knot<'a, K, V>>()
                                           .extended_by_array_unchecked::<(K, V)>(len);
        let new_knot = hold.alloc(layout)?.as_ptr() as *mut Knot<'b, K, V>;
        ptr::write(&mut (*new_knot).hash, (*self).hash);
        ptr::write(&mut (*new_knot).len, len);
        let mut old_leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        let mut new_leaf_ptr = (new_knot as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        let mut i = 0;
        while i < len {
            if let Err(error) = Stow::stow(old_leaf_ptr, new_leaf_ptr, hold) {
                while i > 0 {
                    old_leaf_ptr = old_leaf_ptr.wrapping_sub(1);
                    new_leaf_ptr = new_leaf_ptr.wrapping_sub(1);
                    i = i.wrapping_sub(1);
                    Stow::unstow(old_leaf_ptr, new_leaf_ptr);
                }
                return Err(error);
            }
            old_leaf_ptr = old_leaf_ptr.wrapping_add(1);
            new_leaf_ptr = new_leaf_ptr.wrapping_add(1);
            i = i.wrapping_add(1);
        }
        Ok(new_knot)
    }

    /// Clones this `Knot` into a `hold`.
    unsafe fn clone_tree(self: *mut Knot<'a, K, V>, hold: &dyn Hold<'a>)
        -> Result<*mut Knot<'a, K, V>, HoldError>
        where K: Clone, V: Clone
    {
        let len = (*self).len;
        debug_assert!(len != 0);
        let (layout, leaf_offset) = Layout::for_type::<Knot<'a, K, V>>()
                                           .extended_by_array_unchecked::<(K, V)>(len);
        let new_knot = hold.alloc(layout)?.as_ptr() as *mut Knot<'a, K, V>;
        ptr::write(&mut (*new_knot).hash, (*self).hash);
        ptr::write(&mut (*new_knot).len, len);
        let old_leaf_ptr = (self as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        let new_leaf_ptr = (new_knot as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        let old_slice = slice::from_raw_parts(old_leaf_ptr, len);
        let new_slice = slice::from_raw_parts_mut(new_leaf_ptr, len);
        new_slice.clone_from_slice(old_slice);
        Ok(new_knot)
    }
}

impl<'a, K: Eq, V> Knot<'a, K, V> {
    /// Returns `true` if this `Knot` contains the given `key`.
    unsafe fn contains_key(self: *mut Knot<'a, K, V>, key: &K) -> bool {
        let mut head = self.leaf_array();
        let foot = head.wrapping_add((*self).len);
        while head < foot {
            if &(*head).0 == key {
                return true;
            }
            head = head.wrapping_add(1);
        }
        false
    }

    /// Returns the value associated with the given `key`, or `None` if no
    /// association exists.
    unsafe fn get<'b, 'c>(self: *mut Knot<'a, K, V>, key: &'b K) -> Option<&'c V> {
        let mut head = self.leaf_array();
        let foot = head.wrapping_add((*self).len);
        while head < foot {
            if &(*head).0 == key {
                return Some(&(*head).1);
            }
            head = head.wrapping_add(1);
        }
        None
    }

    /// Associates a new value with a key; leaves the knot in its original
    /// state on allocation failure.
    unsafe fn insert(self: *mut Knot<'a, K, V>, new_key: *const K, new_val: *const V)
        -> KnotInsert<'a, K, V>
    {
        // Get the number of leafs in the old knot.
        let old_len = (*self).len;
        // Keep a pointer to the base address of the old leaf array.
        let old_base = self.leaf_array();
        // Ge a mutable cursor into the old leaf array.
        let mut old_head = old_base;
        // Compute the terminal  pointer of the old leaf array.
        let old_foot = old_base.wrapping_add(old_len);
        // Loop over all leafs in the old leaf array.
        while old_head < old_foot {
            // Check if the head leaf key matches the search key.
            if &(*old_head).0 == &*new_key {
                // Drop the old key.
                ptr::drop_in_place(&mut (*old_head).0);
                // Copy the old value into local memory.
                let old_val = ptr::read(&(*old_head).1);
                // Copy the updated key into the leaf.
                ptr::copy_nonoverlapping(new_key, &mut (*old_head).0, 1);
                // Copy the updated value into new leaf.
                ptr::copy_nonoverlapping(new_val, &mut (*old_head).1, 1);
                // Return the old value.
                return KnotInsert::Diff(old_val);
            }
            // Increment the cursor into the old leaf array.
            old_head = old_head.wrapping_add(1);
        }
        // Key not found in old leaf array.
        // Compute the length of a new knot with the leaf inserted.
        let new_len = old_len.wrapping_add(1);
        // Allocate a new knot, bailing on failure.
        let new_knot = match Knot::alloc(self.holder(), (*self).hash, new_len) {
            // Allocation succeeded.
            Ok(new_knot) => new_knot,
            // Allocation failed; return the error, leaving this knot in its original state.
            Err(error) => return KnotInsert::Fail(error),
        };
        // Get a pointer to the base address of the new leaf array.
        let new_base = new_knot.leaf_array();
        // Copy all leafs from the old leaf array to the new leaf array.
        ptr::copy_nonoverlapping(old_base, new_base, old_len);
        // Get a pointer to the location of the new leaf.
        let new_head = new_base.wrapping_add(old_len);
        // Copy the inserted key into the new leaf.
        ptr::copy_nonoverlapping(new_key, &mut (*new_head).0, 1);
        // Copy the inserted value into the new leaf.
        ptr::copy_nonoverlapping(new_val, &mut (*new_head).1, 1);
        // Return a pointer to the new knot.
        // Caller takes responsibility for deallocating the old knot.
        KnotInsert::Copy(new_knot)
    }

    /// Removes a key from the knot, if found; leaves the knot in its original
    /// state on allocation failure.
    unsafe fn remove(self: *mut Knot<'a, K, V>, key: &K) -> KnotRemove<'a, K, V> {
        // Get the number of leafs in the old knot.
        let old_len = (*self).len;
        // Keep a pointer to the base address of the old leaf array.
        let old_base = self.leaf_array();
        // Get a mutable cursor into the old leaf array.
        let mut old_head = old_base;
        // Compute the terminal ponter of the old leaf array.
        let old_foot = old_base.wrapping_add(old_len);
        // Track the array index of the old_head pointer.
        let mut idx = 0usize;
        // Loop over all leafs in the old leaf array.
        while old_head < old_foot {
            // Check if the head leaf key matches the search key.
            if &(*old_head).0 == &*key {
                // Keys match; remove this leaf.
                // Non-destructively read the leaf to remove.
                let old_leaf = ptr::read(old_head);
                // Compute the length of the a knot with the leaf removed.
                let new_len = old_len.wrapping_sub(1);
                // Check if the new knot will have no leafs.
                if new_len == 0 {
                    // Return the removed leaf, without allocating a replacement knot.
                    // Caller takes responsibility for deallocating the old knot.
                    return KnotRemove::Drop(old_leaf);
                }
                // Check if the new knot will have a single leaf.
                if new_len == 1 {
                    // Non-desctrucitvely read the remaining leaf.
                    let new_leaf = ptr::read(old_base.wrapping_add(1usize.wrapping_sub(idx)));
                    // Return the removed and remaining leafs, without allocating a replacement knot.
                    // Caller takes responsibility for deallocating the old knot.
                    return KnotRemove::Lift(old_leaf, new_leaf);
                }
                // Allocate a new knot with space for the multiple remaining leafs.
                let new_knot = match Knot::alloc(self.holder(), (*self).hash, new_len) {
                    // Allocation succeeded.
                    Ok(new_knot) => new_knot,
                    // Allocation failed; return the error, leaving this knot in its original state.
                    Err(error) => return KnotRemove::Fail(error),
                };
                // Get a pointer to the base address of the new leaf array.
                let new_base = new_knot.leaf_array();
                // Copy the remaining leading leafs from the old leaf array to the new leaf array.
                ptr::copy_nonoverlapping(old_base, new_base, idx);
                // Increment the old_head cursor, past the removed leaf, to the remaining trailing leafs.
                old_head = old_head.wrapping_add(1);
                // Get a pointer to the location of the trailing leafs in the new leaf array.
                let new_head = new_base.wrapping_add(idx);
                // Copy the remaining trailing leafs from the old leaf array to the new leaf array.
                ptr::copy_nonoverlapping(old_head, new_head, new_len.wrapping_sub(idx));
                // Return the new knot with the remaining leafs, along with the removed leaf.
                // Caller takes responsibility for deallocating the old knot.
                return KnotRemove::Copy(old_leaf, new_knot);
            }
            // Increment the cursor into the old leaf array.
            old_head = old_head.wrapping_add(1);
            // Increment the array index of the old_head pointer.
            idx = idx.wrapping_add(1);
        }
        // Key not found.
        KnotRemove::None
    }
}

impl<'a, K, V> IterFrame<'a, K, V> {
    #[inline]
    unsafe fn from_node(node: *mut Node<'a, K, V>) -> IterFrame<'a, K, V> {
        let limb_map = (*node).limb_map;
        let leaf_map = (*node).leaf_map;
        let limb_count = limb_map.count_ones() as usize;
        let layout = Layout::for_type::<Node<'a, K, V>>();
        let (layout, limb_offset) = layout.extended_by_array_unchecked::<*mut Limb<'a, K, V>>(limb_count);
        let leaf_offset = layout.padded_to_type::<(K, V)>().size();
        let limb_ptr = (node as *mut u8).wrapping_add(limb_offset) as *mut *mut Limb<'a, K, V>;
        let leaf_ptr = (node as *mut u8).wrapping_add(leaf_offset) as *mut (K, V);
        IterFrame::Node {
            limb_map: limb_map,
            leaf_map: leaf_map,
            branch: 1,
            limb_ptr: limb_ptr,
            leaf_ptr: leaf_ptr,
        }
    }

    #[inline]
    unsafe fn from_knot(knot: *mut Knot<'a, K, V>) -> IterFrame<'a, K, V> {
        let head_offset = Layout::for_type::<Knot<'a, K, V>>().padded_to_type::<(K, V)>().size();
        let head_ptr = (knot as *mut u8).wrapping_add(head_offset) as *mut (K, V);
        let foot_ptr = head_ptr.wrapping_add((*knot).len);
        IterFrame::Knot {
            head_ptr: head_ptr,
            foot_ptr: foot_ptr,
        }
    }
}

impl<'a, K, V> Clone for IterFrame<'a, K, V> {
    fn clone(&self) -> IterFrame<'a, K, V> {
        match *self {
            IterFrame::Void => IterFrame::Void,
            IterFrame::Node { limb_map, leaf_map, branch, limb_ptr, leaf_ptr } => IterFrame::Node {
                limb_map: limb_map,
                leaf_map: leaf_map,
                branch: branch,
                limb_ptr: limb_ptr,
                leaf_ptr: leaf_ptr,
            },
            IterFrame::Knot { head_ptr, foot_ptr } => IterFrame::Knot {
                head_ptr: head_ptr,
                foot_ptr: foot_ptr,
            }
        }
    }
}

impl<'a, K, V> HashTrieIter<'a, K, V> {
    #[inline]
    pub(crate) fn empty() -> HashTrieIter<'a, K, V> {
        HashTrieIter {
            count: 0,
            depth: -1,
            stack: [IterFrame::Void, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void],
        }
    }

    #[inline]
    fn new(count: usize, base_frame: IterFrame<'a, K, V>) -> HashTrieIter<'a, K, V> {
        HashTrieIter {
            count: count,
            depth: 0,
            stack: [base_frame, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void, IterFrame::Void, IterFrame::Void,
                    IterFrame::Void, IterFrame::Void],
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        return self.count;
    }

    pub(crate) unsafe fn next(&mut self) -> Option<NonNull<(K, V)>> {
        let mut stack_ptr = self.stack.as_mut_ptr().wrapping_add(self.depth as usize);
        while self.depth >= 0 {
            match *stack_ptr {
                IterFrame::Void => return None,
                IterFrame::Node { limb_map, leaf_map, ref mut branch, ref mut limb_ptr, ref mut leaf_ptr } => {
                    if *branch != 0 && (limb_map | leaf_map) & !(*branch - 1) != 0 {
                        let branch_type = BranchType::for_branch(limb_map, leaf_map, *branch);
                        *branch = *branch << 1;
                        match branch_type {
                            BranchType::Void => (),
                            BranchType::Leaf => {
                                let leaf = NonNull::new_unchecked(*leaf_ptr);
                                *leaf_ptr = (*leaf_ptr).wrapping_add(1);
                                return Some(leaf);
                            },
                            BranchType::Node => {
                                let node_ptr = **limb_ptr as *mut Node<'a, K, V>;
                                *limb_ptr = (*limb_ptr).wrapping_add(1);
                                self.depth = self.depth.wrapping_add(1);
                                stack_ptr = stack_ptr.wrapping_add(1);
                                *stack_ptr = IterFrame::from_node(node_ptr);
                            },
                            BranchType::Knot => {
                                let knot_ptr = **limb_ptr as *mut Knot<'a, K, V>;
                                *limb_ptr = (*limb_ptr).wrapping_add(1);
                                self.depth = self.depth.wrapping_add(1);
                                stack_ptr = stack_ptr.wrapping_add(1);
                                *stack_ptr = IterFrame::from_knot(knot_ptr);
                            },
                        };
                    } else {
                        self.depth = self.depth.wrapping_sub(1);
                        *stack_ptr = IterFrame::Void;
                        stack_ptr = stack_ptr.wrapping_sub(1);
                    }
                },
                IterFrame::Knot { ref mut head_ptr, foot_ptr } => {
                    if *head_ptr < foot_ptr {
                        let leaf = NonNull::new_unchecked(*head_ptr);
                        *head_ptr = (*head_ptr).wrapping_add(1);
                        return Some(leaf);
                    } else {
                        self.depth = self.depth.wrapping_sub(1);
                        *stack_ptr = IterFrame::Void;
                        stack_ptr = stack_ptr.wrapping_sub(1);
                    }
                }
            };
        }
        None
    }

    pub(crate) unsafe fn next_back(&mut self) -> Option<NonNull<(K, V)>> {
        let mut stack_ptr = self.stack.as_mut_ptr().wrapping_add(self.depth as usize);
        while self.depth >= 0 {
            match *stack_ptr {
                IterFrame::Void => return None,
                IterFrame::Node { limb_map, leaf_map, ref mut branch, ref mut limb_ptr, ref mut leaf_ptr } => {
                    if *branch != 1 && (limb_map | leaf_map) & (*branch - 1) != 0 {
                        *branch = *branch >> 1;
                        let branch_type = BranchType::for_branch(limb_map, leaf_map, *branch);
                        match branch_type {
                            BranchType::Void => (),
                            BranchType::Leaf => {
                                *leaf_ptr = (*leaf_ptr).wrapping_sub(1);
                                let leaf = NonNull::new_unchecked(*leaf_ptr);
                                return Some(leaf);
                            },
                            BranchType::Node => {
                                *limb_ptr = (*limb_ptr).wrapping_sub(1);
                                let node_ptr = **limb_ptr as *mut Node<'a, K, V>;
                                self.depth = self.depth.wrapping_add(1);
                                stack_ptr = stack_ptr.wrapping_add(1);
                                *stack_ptr = IterFrame::from_node(node_ptr);
                            },
                            BranchType::Knot => {
                                *limb_ptr = (*limb_ptr).wrapping_sub(1);
                                let knot_ptr = **limb_ptr as *mut Knot<'a, K, V>;
                                self.depth = self.depth.wrapping_add(1);
                                stack_ptr = stack_ptr.wrapping_add(1);
                                *stack_ptr = IterFrame::from_knot(knot_ptr);
                            },
                        };
                    } else {
                        self.depth = self.depth.wrapping_sub(1);
                        *stack_ptr = IterFrame::Void;
                        stack_ptr = stack_ptr.wrapping_sub(1);
                    }
                },
                IterFrame::Knot { head_ptr, ref mut foot_ptr } => {
                    if *foot_ptr != head_ptr {
                        *foot_ptr = (*foot_ptr).wrapping_sub(1);
                        let leaf = NonNull::new_unchecked(*foot_ptr);
                        return Some(leaf);
                    } else {
                        self.depth = self.depth.wrapping_sub(1);
                        *stack_ptr = IterFrame::Void;
                        stack_ptr = stack_ptr.wrapping_sub(1);
                    }
                }
            };
        }
        None
    }
}

unsafe impl<'a, K: Send, V: Send> Send for HashTrieIter<'a, K, V> {
}

unsafe impl<'a, K: Sync, V: Sync> Sync for HashTrieIter<'a, K, V> {
}

impl<'a, K: 'a, V: 'a> Clone for HashTrieIter<'a, K, V> {
    fn clone(&self) -> HashTrieIter<'a, K, V> {
        HashTrieIter {
            count: self.count,
            depth: self.depth,
            stack: self.stack.clone(),
        }
    }
}
