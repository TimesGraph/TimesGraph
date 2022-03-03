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

#include <netinet/in.h>
#include <cstdint>
#include <iostream>
#include <optional>
#include <string>

namespace network::kernel {

/**
 * This class represents a network endpoint that is used in Socket.
 * It is used when connecting to an address and to get the current
 * connection address.
 */
struct Endpoint {
  Endpoint();
  Endpoint(std::string ip_address, uint16_t port);

  enum class IpFamily : std::uint8_t { NONE, IP4, IP6 };

  std::string SocketAddress() const;

  bool operator==(const Endpoint &other) const = default;
  friend std::ostream &operator<<(std::ostream &os, const Endpoint &endpoint);

  std::string address;
  uint16_t port{0};
  IpFamily family{IpFamily::NONE};

  /**
   * Tries to parse the given string as either a socket address or ip address.
   * Expected address format:
   *   - "ip_address:port_number"
   *   - "ip_address"
   * We parse the address first. If it's an IP address, a default port must
   * be given, or we return nullopt. If it's a socket address, we try to parse
   * it into an ip address and a port number; even if a default port is given,
   * it won't be used, as we expect that it is given in the address string.
   */
  static std::optional<std::pair<std::string, uint16_t>> ParseSocketOrIpAddress(
      const std::string &address, const std::optional<uint16_t> default_port);

  static IpFamily GetIpFamily(const std::string &ip_address);
};

}  // namespace network::kernel
