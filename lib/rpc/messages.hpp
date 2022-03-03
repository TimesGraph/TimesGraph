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
#include <memory>

#include "utils/typeinfo.hpp"

namespace rpc {

using MessageSize = uint32_t;

/// Each RPC is defined via this struct.
///
/// `TRequest` and `TResponse` are required to be classes which have a static
/// member `kType` of `utils::TypeInfo` type. This is used for proper
/// registration and deserialization of RPC types. Additionally, both `TRequest`
/// and `TResponse` are required to define the following serialization functions:
///   * static void Save(const TRequest|TResponse &, slk::Builder *, ...)
///   * static void Load(TRequest|TResponse *, slk::Reader *, ...)
template <typename TRequest, typename TResponse>
struct RequestResponse {
  using Request = TRequest;
  using Response = TResponse;
};

}  // namespace rpc
