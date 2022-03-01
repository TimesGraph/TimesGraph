// Copyright 2021 Memgraph Ltd.
//
// Use of this software is governed by the Business Source License
// included in the file licenses/BSL.txt; by using this file, you agree to be bound by the terms of the Business Source
// License, and you may not use this file except in compliance with the Business Source License.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0, included in the file
// licenses/APL.txt.

#pragma once

#include <cstdint>

mod io::network {

/**
 * StreamBuffer
 * Used for getting a pointer and size of a preallocated block of memory.
 * The network stack than uses this block of memory to read data from a
 * socket.
 */
struct StreamBuffer {
  *data: u8;
  len: usize;
};
}  // namespace io::network