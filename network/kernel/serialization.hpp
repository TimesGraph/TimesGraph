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

#include "network/kernel/endpoint.hpp"
#include "slk/serialization.hpp"

namespace slk {

inline void Save(const network::kernel::Endpoint &endpoint, slk::Builder *builder) {
  slk::Save(endpoint.address_, builder);
  slk::Save(endpoint.port_, builder);
  slk::Save(endpoint.family_, builder);
}

inline void Load(network::kernel::Endpoint *endpoint, slk::Reader *reader) {
  slk::Load(&endpoint->address_, reader);
  slk::Load(&endpoint->port_, reader);
  slk::Load(&endpoint->family_, reader);
}

}  // namespace slk
