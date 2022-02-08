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

#ifndef VEC_TS_AGG_H
#define VEC_TS_AGG_H

#include "vec_dispatch.h"
#include "rosti.h"

typedef void RostiCount(rosti_t *map, int64_t *p_micros, int64_t count, int32_t valueOffset);

#define ROSTI_DISPATCHER(func)                                                                   \
                                                                                                 \
    RostiCount F_SSE2(func), F_SSE41(func), F_AVX2(func), F_AVX512(func), F_DISPATCH(func);      \
                                                                                                 \
    RostiCount *POINTER_NAME(func) = &func##_dispatch;                                           \
                                                                                                 \
    void F_DISPATCH(func)(rosti_t * map, int64_t * p_micros, int64_t count, int32_t valueOffset) \
    {                                                                                            \
        const int iset = instrset_detect();                                                      \
        if (iset >= 10)                                                                          \
        {                                                                                        \
            POINTER_NAME(func) = &F_AVX512(func);                                                \
        }                                                                                        \
        else if (iset >= 8)                                                                      \
        {                                                                                        \
            POINTER_NAME(func) = &F_AVX2(func);                                                  \
        }                                                                                        \
        else if (iset >= 5)                                                                      \
        {                                                                                        \
            POINTER_NAME(func) = &F_SSE41(func);                                                 \
        }                                                                                        \
        else                                                                                     \
        {                                                                                        \
            POINTER_NAME(func) = &F_SSE2(func);                                                  \
        }                                                                                        \
        (*POINTER_NAME(func))(map, p_micros, count, valueOffset);                                \
    }                                                                                            \
                                                                                                 \
    void func(rosti_t *map, int64_t *p_micros, int64_t count, int32_t valueOffset)               \
    {                                                                                            \
        (*POINTER_NAME(func))(map, p_micros, count, valueOffset);                                \
    }                                                                                            \
    extern "C"                                                                                   \
    {                                                                                            \
        fn Rosti_##func(pRosti                                                                   \
                        : i128, pKeys                                                            \
                        : i128, count                                                            \
                        : i128, valueOffset                                                      \
                        : i64)                                                                   \
        {                                                                                        \
            auto map = reinterpret_cast<rosti_t *>(pRosti);                                      \
            auto *p_micros = reinterpret_cast<int64_t *>(pKeys);                                 \
            func(map, p_micros, count, valueOffset);                                             \
        }                                                                                        \
    }

#endif //VEC_TS_AGG_H
