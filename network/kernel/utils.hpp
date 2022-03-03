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

#include <optional>
#include <string>

#include "network/kernel/endpoint.hpp"

namespace network::kernel {

/// Resolves hostname to ip, if already an ip, just returns it
std::string ResolveHostname(std::string hostname);

/// Gets hostname
std::optional<std::string> GetHostname();

// Try to establish a connection to a remote host
bool CanEstablishConnection(const Endpoint &endpoint);

}  // namespace network::kernel
