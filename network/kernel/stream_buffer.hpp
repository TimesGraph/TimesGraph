/*******************************************************************************
 *
 *
 *  Copyright (c) 2019-2022 TimesGraph
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *
 ******************************************************************************/

#pragma once

#include <cstdint>

namespace network::kernel {

/**
 * StreamBuffer
 * Used for getting a pointer and size of a preallocated block of memory.
 * The network stack than uses this block of memory to read data from a
 * socket.
 */
struct StreamBuffer {
  uint8_t *data;
  size_t len;
};
}  // namespace network::kernel
