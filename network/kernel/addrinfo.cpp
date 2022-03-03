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

#include <netdb.h>
#include <cstring>

#include "io/network/addrinfo.hpp"

#include "io/network/network_error.hpp"

namespace network::kernel {

AddrInfo::AddrInfo(struct addrinfo *info) : info(info) {}

AddrInfo::~AddrInfo() { freeaddrinfo(info); }

AddrInfo AddrInfo::Get(const char *addr, const char *port) {
  struct addrinfo hints;
  memset(&hints, 0, sizeof(struct addrinfo));

  hints.ai_family = AF_UNSPEC;      // IPv4 and IPv6
  hints.ai_socktype = SOCK_STREAM;  // TCP socket
  hints.ai_flags = AI_PASSIVE;

  struct addrinfo *result;
  auto status = getaddrinfo(addr, port, &hints, &result);

  if (status != 0) throw NetworkError(gai_strerror(status));

  return AddrInfo(result);
}

AddrInfo::operator struct addrinfo *() { return info; }
}  // namespace network::kernel
