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

#include "rpc/client.hpp"

namespace rpc {

Client::Client(const network::kernel::Endpoint &endpoint, communication::ClientContext *context)
    : endpoint_(endpoint), context_(context) {}

void Client::Abort() {
  if (!client_) return;
  // We need to call Shutdown on the client to abort any pending read or
  // write operations.
  client_->Shutdown();
  // uninitial optional
  client_ = std::nullopt;
}

}  // namespace rpc
