/*******************************************************************************
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

#ifndef TimesGraph_JIT_IMPL_CONSTS_H
#define TimesGraph_JIT_IMPL_CONSTS_H

#include <cstdint>
#include <limits>

static const int64_t LONG_NULL = std::numeric_limits<int64_t>::min();
static const int32_t INT_NULL = std::numeric_limits<int32_t>::min();

static const double DOUBLE_EPSILON = 0.0000000001;
static const float FLOAT_EPSILON = 0.0000000001;

#endif //TimesGraph_JIT_IMPL_CONSTS_H
