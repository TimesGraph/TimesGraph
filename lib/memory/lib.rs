//! # Dynamic Memory Model
//!
//! An advanced dynamic memory model that scales from resource-constrained
//! embedded devices to high-performance super computers.
//!
//! ## Design goals
//!
//! The dynamic memory model was designed to meet a strict set of requirements:
//!
//! __Bare metal__  
//! No dependence on a system memory allocator; operable out of a statically
//! declared memory region.
//!
//! __Robust__  
//! Handle out-of-memory conditions rigorously, and recoverably.
//!
//! __Deterministic__  
//! Guarantee constant allocation and deallocation times.
//!
//! __Relocatable__  
//! Safely and deterministically move dynamic memory allocations, without the
//! use of a garbage collector.
//!
//! __Scoped__  
//! Constrain dynamic memory allocations to their usage context, by statically
//! bounding memory allocator lifetimes.
//!
//! __Safe__  
//! Conform to Rust's data-race-free ownership model. Statically prevent common
//! memory management errors; dynamically detect memory leaks, and semantic
//! memory model violations.
//!
//! __Low fragmentation__  
//! Prevent memory fragmentation from accruing by relocating and compacting all
//! memory allocations that outlive a usage context.
//!
//! __Good locality__  
//! Maximize data locality by recursively allocating and compacting complex
//! data structures into adjacent cache lines.
//!
//! __Low overhead__  
//! Only a single pointer size of memory overhead per dynamic allocation.
//!
//! __High performance__  
//! Only hundred or so instructions per allocation.
//!
//! __Concurrent__  
//! Thread-safe, lock-free algorithms.
//!
//! __Comprehensive__  
//! Complete set of smart pointer types, including cycle-safe soft references.
//!
//! ## Terminology
//!
//! - _Memory block_: a sequential range of memory addresses.
//! - _Memory object_: a memory block that back-references the allocator from
//!   whence it came.
//! - _Managed pointer_: a pointer-like type that controls the lifetime of a
//!   dynamically allocated memory block.
//! - _Governed pointer_: a pointer-like type that imposes access restrictions
//!   on a memory block.
//! - _Memory lease_: ownership and access restrictions on a particular
//!   reference to a memory block.
//! - _Memory resident_: the usage pattern of a memory block, i.e. single
//!   value, vs. dynamically sized array of values, vs. ring buffer of values.
//! - _Relocation_: the atomic movement of a memory resident from one memory
//!   block to another memory block, after which all managed pointer
//!   dereferences will resolve to the relocated resident.
//! - _Stow_: the recursive relocation of data structure to extend its lifetime
//!   to that of a new allocator.
//!
//! ## Components
//!
//! The implementation breaks down into five major categories:
//!
//! __Physical memory model__  
//! Types that facilitate working with extents of raw memory:
//!
//! - __[`Block`]__: the address and size of a particular memory block.
//! - __[`Layout`]__: size and alignment constraints for a memory block.
//!
//! __Logical memory model__  
//! Traits that generically define the dynamic memory model:
//!
//! - __[`Heap`]__: an abstract memory block allocator.
//! - __[`Hold`]__: an abstract memory object allocator.
//! - __[`Lease`]__: an abstract pointer to a memory block, with associated
//!   metadata.
//! - __[`DynamicLease`]__: an abstract pointer to a resizeable memory block,
//!   with associated metadata.
//! - __[`Resident`]__: a typeclass for generically managing the occupant of a
//!   memory block using associated metadata.
//!
//! __Memory allocators__  
//! Concrete `Heap` and `Hold` allocator implementations:
//!
//! - __[`AddrSpace`]__: Lock-free `Heap` allocator of page-aligned memory
//!   extents from some address range.
//! - __[`Slab`]__: `Heap` allocating from a hunk of memory partitioned into
//!   fixed size memory blocks.
//! - __[`Pack`]__: Pointer-bumping `Hold` allocating from a hunk of memory.
//! - __[`Pool`]__: Pointer-bumping `Hold` allocating from a growable set of
//!   `Heap`-allocated memory hunks.
//!
//! __Memory leases__  
//! Concrete `Lease` implementations:
//!
//! - __[`Raw`]__: a mutably dereferenceable, relocatable, exlusive reference
//!   to a memory resident, with resident metadata stored with the pointer.
//! - __[`Ptr`]__: a mutably dereferenceable, relocatable, exlusive reference
//!   to a memory resident, with resident metadata stored within the allocation.
//! - __[`Mut`]__: a mutably dereferenceable, unrelocatable, strong reference
//!   to a memory resident.
//! - __[`Ref`]__: an immutably dereferenceable, unrelocatable, strong
//!   reference to a memory resident.
//! - __[`Hard`]__: an undereferenceable, relocatable, strong reference to a
//!   memory resident.
//! - __[`Soft`]__: an undereferenceable, relocatable, weak reference to a
//!   memory resident.
//!
//! __Memory residents__  
//! Concrete `Resident` implementations:
//!
//! - __[`Box`]__: stores a single value in a `Lease`.
//! - __[`Buf`]__: stores an array of values in a `Lease`.
//! - __[`String`]__: stores a Unicode code unit sequence in a `Lease`.
//!
//! [`Block`]: block::Block
//! [`Layout`]: block::Layout
//!
//! [`AddrSpace`]: alloc::AddrSpace
//! [`Heap`]: alloc::Heap
//! [`Hold`]: alloc::Hold
//! [`Lease`]: lease::Lease
//! [`DynamicLease`]: lease::DynamicLease
//! [`Resident`]: resident::Resident
//!
//! [`Slab`]: alloc::Slab
//! [`Pack`]: alloc::Pack
//! [`Pool`]: alloc::Pool
//!
//! [`Raw`]: lease::Raw
//! [`Ptr`]: lease::Ptr
//! [`Mut`]: lease::Mut
//! [`Ref`]: lease::Ref
//! [`Hard`]: lease::Hard
//! [`Soft`]: lease::Soft
//!
//! [`Box`]: resident::Box
//! [`Buf`]: resident::Buf
//! [`String`]: resident::String

#![no_std]

#![feature(arbitrary_self_types)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]
#![feature(exact_size_is_empty)]
#![feature(optin_builtin_traits)]
#![feature(specialization)]
#![feature(thread_local)]
#![feature(trusted_len)]

extern crate tg_core;

pub mod block;
pub mod alloc;
pub mod lease;
pub mod resident;
