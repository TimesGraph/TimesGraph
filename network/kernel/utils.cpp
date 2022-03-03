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

#include "network/kernel/utils.hpp"

#include <arpa/inet.h>
#include <netdb.h>

#include <climits>
#include <cstdlib>
#include <cstring>
#include <string>

#include "network/kernel/socket.hpp"

#include "utils/logging.hpp"

namespace network::kernel {

/// Resolves hostname to ip, if already an ip, just returns it
std::string ResolveHostname(std::string hostname) {
  addrinfo hints;
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;  // use AF_INET6 to force IPv6
  hints.ai_socktype = SOCK_STREAM;

  int addr_result;
  addrinfo *servinfo;
  TG_ASSERT((addr_result = getaddrinfo(hostname.c_str(), NULL, &hints, &servinfo)) == 0, "Error with getaddrinfo: {}",
            gai_strerror(addr_result));
  TG_ASSERT(servinfo, "Could not resolve address: {}", hostname);

  std::string address;
  if (servinfo->ai_family == AF_INET) {
    sockaddr_in *hipv4 = (sockaddr_in *)servinfo->ai_addr;
    char astring[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &(hipv4->sin_addr), astring, INET_ADDRSTRLEN);
    address = astring;
  } else {
    sockaddr_in6 *hipv6 = (sockaddr_in6 *)servinfo->ai_addr;
    char astring[INET6_ADDRSTRLEN];
    inet_ntop(AF_INET6, &(hipv6->sin6_addr), astring, INET6_ADDRSTRLEN);
    address = astring;
  }

  freeaddrinfo(servinfo);
  return address;
}

/// Gets hostname
std::optional<std::string> GetHostname() {
  char hostname[HOST_NAME_MAX + 1];
  int result = gethostname(hostname, sizeof(hostname));
  if (result) return std::nullopt;
  return std::string(hostname);
}

bool CanEstablishConnection(const network::kernel::Endpoint &endpoint) {
  network::kernel::Socket client;
  return client.Connect(endpoint);
}

};  // namespace network::kernel
