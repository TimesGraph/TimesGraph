//! # Collection Data Structures

#![no_std]

#![feature(arbitrary_self_types)]
#![feature(dropck_eyepatch)]
#![feature(exact_size_is_empty)]
#![feature(trusted_len)]
#![feature(untagged_unions)]

extern crate tg_core;
extern crate tg_mem;

pub mod hash_trie;
