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

#include "network/kernel/endpoint.hpp"
#include "utils/exceptions.hpp"

namespace rpc {

/// Exception that is thrown whenever a RPC call fails.
/// This exception inherits `std::exception` directly because
/// `utils::BasicException` is used for transient errors that should be reported
/// to the user and `utils::StacktraceException` is used for fatal errors.
/// This exception always requires explicit handling.
class RpcFailedException final : public utils::BasicException {
 public:
  RpcFailedException(const io::network::Endpoint &endpoint)
      : utils::BasicException::BasicException(
            "Couldn't communicate with the cluster! Please contact your "
            "timesgraph administrator."),
        endpoint_(endpoint) {}

  /// Returns the endpoint associated with the error.
  const io::network::Endpoint &endpoint() const { return endpoint_; }

 private:
  io::network::Endpoint endpoint_;
};
}  // namespace rpc
