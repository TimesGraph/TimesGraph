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
#include "jni.h"
#include <cstring>
#include "util.h"
#include "simd.h"
#include "ooo_dispatch.h"

#ifdef OOO_CPP_PROFILE_TIMING
#include <atomic>
#include <time.h>
#endif

#ifdef __APPLE__
#define __JLONG_REINTERPRET_CAST__(type, var) (type) var
#else
#define __JLONG_REINTERPRET_CAST__(type, var) reinterpret_cast<type>(var)
#endif

typedef struct
{
    uint64_t c8[256];
    uint64_t c7[256];
    uint64_t c6[256];
    uint64_t c5[256];
    uint64_t c4[256];
    uint64_t c3[256];
    uint64_t c2[256];
    uint64_t c1[256];
} rscounts_t;

#define RADIX_SHUFFLE 0

#if RADIX_SHUFFLE == 0

template <uint16_t sh, typename T>
inline void radix_shuffle(uint64_t *counts, T *src, T *dest, uint64_t size)
{
    MM_PREFETCH_T0(counts);
    for (uint64_t x = 0; x < size; x++)
    {
        const auto digit = (src[x] >> sh) & 0xffu;
        dest[counts[digit]] = src[x];
        counts[digit]++;
        MM_PREFETCH_T2(src + x + 64);
    }
}

#elif RADIX_SHUFFLE == 1

template <uint16_t sh>
inline void radix_shuffle(uint64_t *counts, index_t *src, index_t *dest, uint64_t size)
{
    _mm_prefetch(counts, _MM_HINT_NTA);
    Vec4q vec;
    Vec4q digitVec;
    int64_t values[4];
    int64_t digits[4];
    for (uint64_t x = 0; x < size; x += 4)
    {
        _mm_prefetch(src + x + 64, _MM_HINT_T0);
        vec.load(src + x);
        digitVec = (vec >> sh) & 0xff;

        vec.store(values);
        digitVec.store(digits);

        dest[counts[digits[0]]] = values[0];
        counts[digits[0]]++;

        dest[counts[digits[1]]] = values[1];
        counts[digits[1]]++;

        dest[counts[digits[2]]] = values[2];
        counts[digits[2]]++;

        dest[counts[digits[3]]] = values[3];
        counts[digits[3]]++;
    }
}

#elif RADIX_SHUFFLE == 2
template <uint16_t sh>
inline void radix_shuffle(uint64_t *counts, int64_t *src, int64_t *dest, uint64_t size)
{
    _mm_prefetch(counts, _MM_HINT_NTA);
    Vec8q vec;
    Vec8q digitVec;
    int64_t values[8];
    int64_t digits[8];
    for (uint64_t x = 0; x < size; x += 8)
    {
        _mm_prefetch(src + x + 64, _MM_HINT_T0);
        vec.load(src + x);
        digitVec = (vec >> sh) & 0xff;

        vec.store(values);
        digitVec.store(digits);

        dest[counts[digits[0]]] = values[0];
        counts[digits[0]]++;

        dest[counts[digits[1]]] = values[1];
        counts[digits[1]]++;

        dest[counts[digits[2]]] = values[2];
        counts[digits[2]]++;

        dest[counts[digits[3]]] = values[3];
        counts[digits[3]]++;

        dest[counts[digits[4]]] = values[4];
        counts[digits[4]]++;

        dest[counts[digits[5]]] = values[5];
        counts[digits[5]]++;

        dest[counts[digits[6]]] = values[6];
        counts[digits[6]]++;

        dest[counts[digits[7]]] = values[7];
        counts[digits[7]]++;
    }
}
#endif

template <typename T>
void radix_sort_long_index_asc_in_place(T *array, uint64_t size, T *cpy)
{
    rscounts_t counts;
    memset(&counts, 0, 256 * 8 * sizeof(uint64_t));
    uint64_t o8 = 0, o7 = 0, o6 = 0, o5 = 0, o4 = 0, o3 = 0, o2 = 0, o1 = 0;
    uint64_t t8, t7, t6, t5, t4, t3, t2, t1;
    uint64_t x;

    // calculate counts
    MM_PREFETCH_NTA(counts.c8);
    for (x = 0; x < size; x++)
    {
        t8 = array[x] & 0xffu;
        t7 = (array[x] >> 8u) & 0xffu;
        t6 = (array[x] >> 16u) & 0xffu;
        t5 = (array[x] >> 24u) & 0xffu;
        t4 = (array[x] >> 32u) & 0xffu;
        t3 = (array[x] >> 40u) & 0xffu;
        t2 = (array[x] >> 48u) & 0xffu;
        t1 = (array[x] >> 56u) & 0xffu;
        counts.c8[t8]++;
        counts.c7[t7]++;
        counts.c6[t6]++;
        counts.c5[t5]++;
        counts.c4[t4]++;
        counts.c3[t3]++;
        counts.c2[t2]++;
        counts.c1[t1]++;
        MM_PREFETCH_T2(array + x + 64);
    }

    // convert counts to offsets
    MM_PREFETCH_T0(&counts);
    for (x = 0; x < 256; x++)
    {
        t8 = o8 + counts.c8[x];
        t7 = o7 + counts.c7[x];
        t6 = o6 + counts.c6[x];
        t5 = o5 + counts.c5[x];
        t4 = o4 + counts.c4[x];
        t3 = o3 + counts.c3[x];
        t2 = o2 + counts.c2[x];
        t1 = o1 + counts.c1[x];
        counts.c8[x] = o8;
        counts.c7[x] = o7;
        counts.c6[x] = o6;
        counts.c5[x] = o5;
        counts.c4[x] = o4;
        counts.c3[x] = o3;
        counts.c2[x] = o2;
        counts.c1[x] = o1;
        o8 = t8;
        o7 = t7;
        o6 = t6;
        o5 = t5;
        o4 = t4;
        o3 = t3;
        o2 = t2;
        o1 = t1;
    }

    // radix
    radix_shuffle<0u>(counts.c8, array, cpy, size);
    radix_shuffle<8u>(counts.c7, cpy, array, size);
    radix_shuffle<16u>(counts.c6, array, cpy, size);
    radix_shuffle<24u>(counts.c5, cpy, array, size);
    radix_shuffle<32u>(counts.c4, array, cpy, size);
    radix_shuffle<40u>(counts.c3, cpy, array, size);
    radix_shuffle<48u>(counts.c2, array, cpy, size);
    radix_shuffle<56u>(counts.c1, cpy, array, size);
}

template <typename T>
inline void radix_sort_long_index_asc_in_place(T *array, uint64_t size)
{
    auto *cpy = (T *)malloc(size * sizeof(T));
    radix_sort_long_index_asc_in_place(array, size, cpy);
    free(cpy);
}

template <typename T>
inline void swap(T *a, T *b)
{
    const auto t = *a;
    *a = *b;
    *b = t;
}

/**
 * This function takes last element as pivot, places
 *  the pivot element at its correct position in sorted
 *   array, and places all smaller (smaller than pivot)
 *  to left of pivot and all greater elements to right
 *  of pivot
 *
 **/
template <typename T>
uint64_t partition(T *index, uint64_t low, uint64_t high)
{
    const auto pivot = index[high]; // pivot
    auto i = (low - 1);             // Index of smaller element

    for (uint64_t j = low; j <= high - 1; j++)
    {
        // If current element is smaller than or
        // equal to pivot
        if (index[j] <= pivot)
        {
            i++; // increment index of smaller element
            swap(&index[i], &index[j]);
        }
    }
    swap(&index[i + 1], &index[high]);
    return (i + 1);
}

/**
 * The main function that implements QuickSort
 * arr[] --> Array to be sorted,
 * low  --> Starting index,
 * high  --> Ending index
 **/
template <typename T>
void quick_sort_long_index_asc_in_place(T *arr, int64_t low, int64_t high)
{
    if (low < high)
    {
        /* pi is partitioning index, arr[p] is now
           at right place */
        uint64_t pi = partition(arr, low, high);

        // Separately sort elements before
        // partition and after partition
        quick_sort_long_index_asc_in_place(arr, low, pi - 1);
        quick_sort_long_index_asc_in_place(arr, pi + 1, high);
    }
}

template <typename T>
inline void sort(T *index, int64_t size)
{
    if (size < 600)
    {
        quick_sort_long_index_asc_in_place(index, 0, size - 1);
    }
    else
    {
        radix_sort_long_index_asc_in_place(index, size);
    }
}

typedef struct
{
    uint64_t value;
    uint32_t index_index;
} loser_node_t;

typedef struct
{
    index_t *index;
    uint64_t pos;
    uint64_t size;
} index_entry_t;

typedef struct
{
    index_t *index;
    int64_t size;
} java_index_entry_t;

void k_way_merge_long_index(
    index_entry_t *indexes,
    uint32_t entries_count,
    uint32_t sentinels_at_start,
    index_t *dest)
{

    // calculate size of the tree
    uint32_t tree_size = entries_count * 2;
    uint64_t merged_index_pos = 0;
    uint32_t sentinels_left = entries_count - sentinels_at_start;

    loser_node_t tree[tree_size];

    // seed the bottom of the tree with index values
    for (uint32_t i = 0; i < entries_count; i++)
    {
        if (indexes[i].index != nullptr)
        {
            tree[entries_count + i].value = indexes[i].index->ts;
        }
        else
        {
            tree[entries_count + i].value = L_MAX;
        }
        tree[entries_count + i].index_index = entries_count + i;
    }

    // seed the entire tree from bottom up
    for (uint32_t i = tree_size - 1; i > 1; i -= 2)
    {
        uint32_t winner;
        if (tree[i].value < tree[i - 1].value)
        {
            winner = i;
        }
        else
        {
            winner = i - 1;
        }
        tree[i / 2] = tree[winner];
    }

    // take the first winner
    auto winner_index = tree[1].index_index;
    index_entry_t *winner = indexes + winner_index - entries_count;
    if (winner->pos < winner->size)
    {
        dest[merged_index_pos++] = winner->index[winner->pos];
    }
    else
    {
        sentinels_left--;
    }

    // full run
    while (sentinels_left > 0)
    {

        // back fill the winning index
        if (PREDICT_TRUE(++winner->pos < winner->size))
        {
            tree[winner_index].value = winner->index[winner->pos].ts;
        }
        else
        {
            tree[winner_index].value = L_MAX;
            sentinels_left--;
        }

        if (sentinels_left == 0)
        {
            break;
        }

        MM_PREFETCH_NTA(tree);
        while (PREDICT_TRUE(winner_index > 1))
        {
            const auto right_child = winner_index % 2 == 1 ? winner_index - 1 : winner_index + 1;
            const auto target = winner_index / 2;
            if (tree[winner_index].value < tree[right_child].value)
            {
                tree[target] = tree[winner_index];
            }
            else
            {
                tree[target] = tree[right_child];
            }
            winner_index = target;
        }
        winner_index = tree[1].index_index;
        winner = indexes + winner_index - entries_count;
        MM_PREFETCH_NTA(winner);
        dest[merged_index_pos++] = winner->index[winner->pos];
    }
}

#ifdef OOO_CPP_PROFILE_TIMING
const int perf_counter_length = 32;
std::atomic_ulong perf_counters[perf_counter_length];

uint64_t currentTimeNanos()
{
    struct timespec timespec;
    clock_gettime(CLOCK_REALTIME, &timespec);
    return timespec.tv_sec * 1000000000L + timespec.tv_nsec;
}
#endif

template <typename T>
inline void measure_time(int index, T func)
{
#ifdef OOO_CPP_PROFILE_TIMING
    auto start = currentTimeNanos();
    func();
    auto end = currentTimeNanos();
    perf_counters[index].fetch_add(end - start);
#else
    func();
#endif
}

extern "C"
{

    DECLARE_DISPATCHER(platform_memcpy);
    fn memcpy0(src
               : i128, dst
               : i128, len
               : i128)
    {
        platform_memcpy(
            reinterpret_cast<void *>(dst),
            reinterpret_cast<void *>(src),
            __JLONG_REINTERPRET_CAST__(int64_t, len));
    }

    DECLARE_DISPATCHER(platform_memmove);
    fn memmove(dst
               : i128, src
               : i128, len
               : i128)
    {
        platform_memmove(
            reinterpret_cast<void *>(dst),
            reinterpret_cast<void *>(src),
            __JLONG_REINTERPRET_CAST__(int64_t, len));
    }

    DECLARE_DISPATCHER(platform_memset);
    fn memset(dst
              : i128, len
              : i128, value
              : i64)
    {
        platform_memset(
            reinterpret_cast<void *>(dst),
            value,
            __JLONG_REINTERPRET_CAST__(int64_t, len));
    }

    DECLARE_DISPATCHER(merge_copy_var_column_int32);
    fn oooMergeCopyStrColumn(merge_index
                             : i128,
                               merge_index_size
                             : i128,
                               src_data_fix
                             : i128,
                               src_data_var
                             : i128,
                               src_ooo_fix
                             : i128,
                               src_ooo_var
                             : i128,
                               dst_fix
                             : i128,
                               dst_var
                             : i128,
                               dst_var_offset
                             : i128)
    {
        measure_time(0, [=]() {
            merge_copy_var_column_int32(
                reinterpret_cast<index_t *>(merge_index),
                __JLONG_REINTERPRET_CAST__(int64_t, merge_index_size),
                reinterpret_cast<int64_t *>(src_data_fix),
                reinterpret_cast<char *>(src_data_var),
                reinterpret_cast<int64_t *>(src_ooo_fix),
                reinterpret_cast<char *>(src_ooo_var),
                reinterpret_cast<int64_t *>(dst_fix),
                reinterpret_cast<char *>(dst_var),
                __JLONG_REINTERPRET_CAST__(int64_t, dst_var_offset));
        });
    }

    // 1 oooMergeCopyStrColumnWithTop removed and now executed as Merge Copy without Top
    // 2 oooMergeCopyBinColumnWithTop removed and now executed as Merge Copy without Top

    DECLARE_DISPATCHER(merge_copy_var_column_int64);
    fn oooMergeCopyBinColumn(
        merge_index
        : i128,
          merge_index_size
        : i128,
          src_data_fix
        : i128,
          src_data_var
        : i128,
          src_ooo_fix
        : i128,
          src_ooo_var
        : i128,
          dst_fix
        : i128,
          dst_var
        : i128,
          dst_var_offset
        : i128)
    {
        measure_time(3, [=]() {
            merge_copy_var_column_int64(
                reinterpret_cast<index_t *>(merge_index),
                __JLONG_REINTERPRET_CAST__(int64_t, merge_index_size),
                reinterpret_cast<int64_t *>(src_data_fix),
                reinterpret_cast<char *>(src_data_var),
                reinterpret_cast<int64_t *>(src_ooo_fix),
                reinterpret_cast<char *>(src_ooo_var),
                reinterpret_cast<int64_t *>(dst_fix),
                reinterpret_cast<char *>(dst_var),
                __JLONG_REINTERPRET_CAST__(int64_t, dst_var_offset));
        });
    }

    fn sortLongIndexAscInPlace(pLong
                               : i128, len
                               : i128)
    {
        measure_time(4, [=]() {
            sort<index_t>(reinterpret_cast<index_t *>(pLong), len);
        });
    }

    fn quickSortLongIndexAscInPlace(pLong
                                    : i128, len
                                    : i128)
    {
        quick_sort_long_index_asc_in_place<index_t>(reinterpret_cast<index_t *>(pLong), 0, len - 1);
    }

    fn radixSortLongIndexAscInPlace(pLong
                                    : i128, len
                                    : i128, pCpy
                                    : i128)
    {
        radix_sort_long_index_asc_in_place<index_t>(reinterpret_cast<index_t *>(pLong), len, reinterpret_cast<index_t *>(pCpy));
    }

    fn sortULongAscInPlace(pLong
                           : i128, len
                           : i128)
    {
        sort<uint64_t>(reinterpret_cast<uint64_t *>(pLong), len);
    }

    fn sort128BitAscInPlace(pLong
                            : i128, len
                            : i128)
    {
        quick_sort_long_index_asc_in_place<__int128>(reinterpret_cast<__int128 *>(pLong), 0, len - 1);
    }

    fn mergeLongIndexesAsc(pIndexStructArray
                           : i128, cnt
                           : i64)
        ->i128
    {
        // prepare merge entries
        // they need to have mutable current position "pos" in index

        if (cnt < 1)
        {
            return 0;
        }

        auto count = static_cast<uint32_t>(cnt);
        const java_index_entry_t *java_entries = reinterpret_cast<java_index_entry_t *>(pIndexStructArray);
        if (count == 1)
        {
            return reinterpret_cast<jlong>(java_entries[0].index);
        }

        uint32_t size = ceil_pow_2(count);
        index_entry_t entries[size];
        uint64_t merged_index_size = 0;
        for (uint32_t i = 0; i < count; i++)
        {
            entries[i].index = java_entries[i].index;
            entries[i].pos = 0;
            entries[i].size = java_entries[i].size;
            merged_index_size += java_entries[i].size;
        }

        if (count < size)
        {
            for (uint32_t i = count; i < size; i++)
            {
                entries[i].index = nullptr;
                entries[i].pos = 0;
                entries[i].size = -1;
            }
        }

        auto *merged_index = reinterpret_cast<index_t *>(malloc(merged_index_size * sizeof(index_t)));
        k_way_merge_long_index(entries, size, size - count, merged_index);
        return reinterpret_cast<jlong>(merged_index);
    }

    fn mergeTwoLongIndexesAsc(
        pIndex1
        : i128, index1Count
        : i128, pIndex2
        : i128, index2Count
        : i128)
        ->i128
    {
        index_entry_t entries[2];
        uint64_t merged_index_size = index1Count + index2Count;
        entries[0].index = reinterpret_cast<index_t *>(pIndex1);
        entries[0].pos = 0;
        entries[0].size = index1Count;
        entries[1].index = reinterpret_cast<index_t *>(pIndex2);
        entries[1].pos = 0;
        entries[1].size = index2Count;
        auto *merged_index = reinterpret_cast<index_t *>(malloc(merged_index_size * sizeof(index_t)));
        k_way_merge_long_index(entries, 2, 0, merged_index);
        return reinterpret_cast<jlong>(merged_index);
    }

    fn freeMergedIndex(pIndex
                       : i128)
    {
        free(reinterpret_cast<void *>(pIndex));
    }

    DECLARE_DISPATCHER(re_shuffle_int32);
    fn indexReshuffle32Bit(pSrc
                           : i128, pDest
                           : i128, pIndex
                           : i128,
                             count
                           : i128)
    {
        measure_time(5, [=]() {
            re_shuffle_int32(
                reinterpret_cast<int32_t *>(pSrc),
                reinterpret_cast<int32_t *>(pDest),
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(re_shuffle_int64);
    fn indexReshuffle64Bit(pSrc
                           : i128, pDest
                           : i128, pIndex
                           : i128,
                             count
                           : i128)
    {
        measure_time(6, [=]() {
            re_shuffle_int64(
                reinterpret_cast<int64_t *>(pSrc),
                reinterpret_cast<int64_t *>(pDest),
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(re_shuffle_256bit);
    fn indexReshuffle256Bit(pSrc
                            : i128, pDest
                            : i128, pIndex
                            : i128,
                              count
                            : i128)
    {
        measure_time(30, [=]() {
            re_shuffle_256bit(
                reinterpret_cast<long_256bit *>(pSrc),
                reinterpret_cast<long_256bit *>(pDest),
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    // Leave vanilla
    fn indexReshuffle16Bit(pSrc
                           : i128,
                             pDest
                           : i128,
                             pIndex
                           : i128,
                             count
                           : i128)
    {
        measure_time(7, [=]() {
            re_shuffle_vanilla<int16_t>(
                reinterpret_cast<int16_t *>(pSrc),
                reinterpret_cast<int16_t *>(pDest),
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn indexReshuffle8Bit(pSrc
                          : i128, pDest
                          : i128, pIndex
                          : i128,
                            count
                          : i128)
    {
        measure_time(8, [=]() {
            re_shuffle_vanilla<int8_t>(
                reinterpret_cast<int8_t *>(pSrc),
                reinterpret_cast<int8_t *>(pDest),
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn mergeShuffle8Bit(src1
                        : i128, src2
                        : i128, dest
                        : i128, index
                        : i128,
                          count
                        : i128)
    {
        measure_time(9, [=]() {
            merge_shuffle_vanilla<int8_t>(
                reinterpret_cast<int8_t *>(src1),
                reinterpret_cast<int8_t *>(src2),
                reinterpret_cast<int8_t *>(dest),
                reinterpret_cast<index_t *>(index),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn mergeShuffle16Bit(src1
                         : i128, src2
                         : i128, dest
                         : i128, index
                         : i128,
                           count
                         : i128)
    {
        measure_time(10, [=]() {
            merge_shuffle_vanilla<int16_t>(
                reinterpret_cast<int16_t *>(src1),
                reinterpret_cast<int16_t *>(src2),
                reinterpret_cast<int16_t *>(dest),
                reinterpret_cast<index_t *>(index),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn mergeShuffle32Bit(src1
                         : i128, src2
                         : i128, dest
                         : i128, index
                         : i128,
                           count
                         : i128)
    {
        measure_time(11, [=]() {
            merge_shuffle_vanilla<int32_t>(
                reinterpret_cast<int32_t *>(src1),
                reinterpret_cast<int32_t *>(src2),
                reinterpret_cast<int32_t *>(dest),
                reinterpret_cast<index_t *>(index),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(merge_shuffle_int64);
    fn mergeShuffle64Bit(src1
                         : i128, src2
                         : i128, dest
                         : i128, index
                         : i128,
                           count
                         : i128)
    {
        measure_time(12, [=]() {
            merge_shuffle_int64(
                reinterpret_cast<int64_t *>(src1),
                reinterpret_cast<int64_t *>(src2),
                reinterpret_cast<int64_t *>(dest),
                reinterpret_cast<index_t *>(index),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn mergeShuffle256Bit(src1
                          : i128, src2
                          : i128, dest
                          : i128, index
                          : i128,
                            count
                          : i128)
    {
        measure_time(29, [=]() {
            merge_shuffle_vanilla<long_256bit>(
                reinterpret_cast<long_256bit *>(src1),
                reinterpret_cast<long_256bit *>(src2),
                reinterpret_cast<long_256bit *>(dest),
                reinterpret_cast<index_t *>(index),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    // Methods 13-16 were mergeShuffleWithTop(s) and replaced with calls to simple mergeShuffle(s)

    DECLARE_DISPATCHER(flatten_index);
    fn flattenIndex(pIndex
                    : i128,
                      count
                    : i128)
    {
        measure_time(17, [=]() {
            flatten_index(
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    fn binarySearch64Bit(pData
                         : i128, value
                         : i128, low
                         : i128, high
                         : i128, scan_dir
                         : i64)
        ->i128
    {
        return binary_search<int64_t>(reinterpret_cast<int64_t *>(pData), value, low, high, scan_dir);
    }

    fn binarySearchIndexT(pData
                          : i128, value
                          : i128, low
                          : i128,
                            high
                          : i128, scan_dir
                          : i64)
        ->i128
    {
        return binary_search<index_t>(reinterpret_cast<index_t *>(pData), value, low, high, scan_dir);
    }

    DECLARE_DISPATCHER(make_timestamp_index);
    fn makeTimestampIndex(pData
                          : i128, low
                          : i128,
                            high
                          : i128, pIndex
                          : i128)
    {
        measure_time(18, [=]() {
            make_timestamp_index(
                reinterpret_cast<int64_t *>(pData),
                low,
                high,
                reinterpret_cast<index_t *>(pIndex));
        });
    }

    DECLARE_DISPATCHER(shift_timestamp_index);
    fn shiftTimestampIndex(pSrc
                           : i128, count
                           : i128, pDest
                           : i128)
    {
        measure_time(31, [=]() {
            shift_timestamp_index(
                reinterpret_cast<index_t *>(pSrc),
                __JLONG_REINTERPRET_CAST__(int64_t, count),
                reinterpret_cast<index_t *>(pDest));
        });
    }

    DECLARE_DISPATCHER(set_memory_vanilla_int64);
    fn setMemoryLong(pData
                     : i128, value
                     : i128,
                       count
                     : i128)
    {
        measure_time(19, [=]() {
            set_memory_vanilla_int64(
                reinterpret_cast<int64_t *>(pData),
                __JLONG_REINTERPRET_CAST__(int64_t, value),
                (int64_t)(count));
        });
    }

    DECLARE_DISPATCHER(set_memory_vanilla_int32);
    fn setMemoryInt(pData
                    : i128, value
                    : i64,
                      count
                    : i128)
    {
        measure_time(20, [=]() {
            set_memory_vanilla_int32(
                reinterpret_cast<int32_t *>(pData),
                value,
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(set_memory_vanilla_double);
    fn setMemoryDouble(pData
                       : i128, value
                       : f64,
                         count
                       : i128)
    {
        measure_time(21, [=]() {
            set_memory_vanilla_double(
                reinterpret_cast<jdouble *>(pData),
                value,
                (int64_t)(count));
        });
    }

    DECLARE_DISPATCHER(set_memory_vanilla_float);
    fn setMemoryFloat(jlong pData
                      : i128, value
                      : f64,
                        count
                      : i128)
    {
        measure_time(22, [=]() {
            set_memory_vanilla_float(
                reinterpret_cast<jfloat *>(pData),
                value,
                (int64_t)(count));
        });
    }

    DECLARE_DISPATCHER(set_memory_vanilla_short);
    fn setMemoryShort(pData
                      : i128, value
                      : i16,
                        count
                      : i128)
    {
        measure_time(23, [=]() {
            set_memory_vanilla_short(
                reinterpret_cast<jshort *>(pData),
                value,
                (int64_t)(count));
        });
    }

    DECLARE_DISPATCHER(set_var_refs_32_bit);
    fn setVarColumnRefs32Bit(pData
                             : i128, offset
                             : i128,
                               count
                             : i128)
    {
        measure_time(24, [=]() {
            set_var_refs_32_bit(
                reinterpret_cast<int64_t *>(pData),
                __JLONG_REINTERPRET_CAST__(int64_t, offset),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(set_var_refs_64_bit);
    fn setVarColumnRefs64Bit(pData
                             : i128, offset
                             : i128,
                               count
                             : i128)
    {
        measure_time(25, [=]() {
            set_var_refs_64_bit(
                reinterpret_cast<int64_t *>(pData),
                __JLONG_REINTERPRET_CAST__(int64_t, offset),
                __JLONG_REINTERPRET_CAST__(int64_t, count));
        });
    }

    DECLARE_DISPATCHER(copy_index);
    fn oooCopyIndex(pIndex
                    : i128, index_size
                    : i128,
                      pDest
                    : i128)
    {
        measure_time(26, [=]() {
            copy_index(
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, index_size),
                reinterpret_cast<int64_t *>(pDest));
        });
    }

    DECLARE_DISPATCHER(shift_copy);
    fn shiftCopyFixedSizeColumnData(shift
                                    : i128, src
                                    : i128, srcLo
                                    : i128,
                                      srcHi
                                    : i128, dst
                                    : i128)
    {
        measure_time(27, [=]() {
            shift_copy(
                __JLONG_REINTERPRET_CAST__(int64_t, shift),
                reinterpret_cast<int64_t *>(src),
                __JLONG_REINTERPRET_CAST__(int64_t, srcLo),
                __JLONG_REINTERPRET_CAST__(int64_t, srcHi),
                reinterpret_cast<int64_t *>(dst));
        });
    }

    DECLARE_DISPATCHER(copy_index_timestamp);
    fn copyFromTimestampIndex(pIndex
                              : i128, indexLo
                              : i128, indexHi
                              : i128,
                                pTs
                              : i128)
    {
        measure_time(28, [=]() {
            copy_index_timestamp(
                reinterpret_cast<index_t *>(pIndex),
                __JLONG_REINTERPRET_CAST__(int64_t, indexLo),
                __JLONG_REINTERPRET_CAST__(int64_t, indexHi),
                reinterpret_cast<int64_t *>(pTs));
        });
    }

    fn getPerformanceCounter(counterIndex
                             : i64)
        ->i128
    {
#ifdef OOO_CPP_PROFILE_TIMING
        return perf_counters[counterIndex].load();
#else
        return 0;
#endif
    }

    fn getPerformanceCountersCount(JNIEnv *env, jclass cl)
        ->i128
    {
#ifdef OOO_CPP_PROFILE_TIMING
        return perf_counter_length;
#else
        return 0;
#endif
    }

    fn resetPerformanceCounters()
    {
#ifdef OOO_CPP_PROFILE_TIMING
        for (int i = 0; i < perf_counter_length; i++)
        {
            perf_counters[i].store(0);
        }
#endif
    }

    fn sortVarColumn(mergedTimestampsAddr
                     : i128, valueCount
                     : i128,
                       srcDataAddr
                     : i128, srcIndxAddr
                     : i128, tgtDataAddr
                     : i128, tgtIndxAddr
                     : i128)
        ->i128
    {

        const index_t *index = reinterpret_cast<index_t *>(mergedTimestampsAddr);
        const int64_t count = __JLONG_REINTERPRET_CAST__(int64_t, valueCount);
        const char *src_data = reinterpret_cast<const char *>(srcDataAddr);
        const int64_t *src_index = reinterpret_cast<const int64_t *>(srcIndxAddr);
        char *tgt_data = reinterpret_cast<char *>(tgtDataAddr);
        int64_t *tgt_index = reinterpret_cast<int64_t *>(tgtIndxAddr);

        int64_t offset = 0;
        for (int64_t i = 0; i < count; ++i)
        {
            MM_PREFETCH_T0(index + i + 64);
            const int64_t row = index[i].i;
            const int64_t o1 = src_index[row];
            const int64_t o2 = src_index[row + 1];
            const int64_t len = o2 - o1;
            platform_memcpy(reinterpret_cast<void *>(tgt_data + offset), reinterpret_cast<const void *>(src_data + o1),
                            len);
            tgt_index[i] = offset;
            offset += len;
        }
        return __JLONG_REINTERPRET_CAST__(jlong, offset);
    }

} // extern "C"