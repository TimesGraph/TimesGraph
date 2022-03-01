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

#include "kvstore/kvstore.hpp"

#include "utils/file.hpp"
#include "utils/logging.hpp"

namespace kvstore
{

  struct KVStore::impl
  {
  };

  KVStore::KVStore(std::filesystem::path storage) {}

  KVStore::~KVStore() {}
  // 放入值
  bool KVStore::Put(const std::string &key, const std::string &value)
  {
    LOG_FATAL("Unsupported operation (KVStore::Put) -- this is a dummy kvstore");
  }
  // 放入多值
  bool KVStore::PutMultiple(const std::map<std::string, std::string> &items)
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::PutMultiple) -- this is a "
        "dummy kvstore");
  }
  // 获取值
  std::optional<std::string> KVStore::Get(const std::string &key) const noexcept
  {
    LOG_FATAL("Unsupported operation (KVStore::Get) -- this is a dummy kvstore");
  }
  // 删除值
  bool KVStore::Delete(const std::string &key)
  {
    LOG_FATAL("Unsupported operation (KVStore::Delete) -- this is a dummy kvstore");
  }
  // 删除多值
  bool KVStore::DeleteMultiple(const std::vector<std::string> &keys)
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::DeleteMultiple) -- this is "
        "a dummy kvstore");
  }
  // 删除前缀
  bool KVStore::DeletePrefix(const std::string &prefix)
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::DeletePrefix) -- this is a "
        "dummy kvstore");
  }
  // 删除多个前缀
  bool KVStore::PutAndDeleteMultiple(const std::map<std::string, std::string> &items,
                                     const std::vector<std::string> &keys)
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::PutAndDeleteMultiple) -- this is a "
        "dummy kvstore");
  }

  // iterator

  struct KVStore::iterator::impl
  {
  };
  // 迭代
  KVStore::iterator::iterator(const KVStore *kvstore, const std::string &prefix, bool at_end) : pimpl_(new impl()) {}
  // 迭代
  KVStore::iterator::iterator(KVStore::iterator &&other) { pimpl_ = std::move(other.pimpl_); }
  // 迭代
  KVStore::iterator::~iterator() {}

  KVStore::iterator &KVStore::iterator::operator=(KVStore::iterator &&other)
  {
    pimpl_ = std::move(other.pimpl_);
    return *this;
  }

  KVStore::iterator &KVStore::iterator::operator++()
  {
    LOG_FATAL(
        "Unsupported operation (&KVStore::iterator::operator++) -- "
        "this is a dummy kvstore");
  }

  bool KVStore::iterator::operator==(const iterator &other) const { return true; }

  bool KVStore::iterator::operator!=(const iterator &other) const { return false; }

  KVStore::iterator::reference KVStore::iterator::operator*()
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::iterator::operator*)-- this "
        "is a dummy kvstore");
  }

  KVStore::iterator::pointer KVStore::iterator::operator->()
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::iterator::operator->) -- "
        "this is a dummy kvstore");
  }

  void KVStore::iterator::SetInvalid() {}

  bool KVStore::iterator::IsValid() { return false; }
  // 大小
  size_t KVStore::Size(const std::string &prefix) { return 0; }
  // 压缩范围
  bool KVStore::CompactRange(const std::string &begin_prefix, const std::string &end_prefix)
  {
    LOG_FATAL(
        "Unsupported operation (KVStore::Compact) -- this is a "
        "dummy kvstore");
  }

} // namespace kvstore
