#include <jni.h>
#include <cstdint>
#include "asmlib/asmlib.h"

extern "C"
{

#ifdef __APPLE__
        fn Os_compareAndSwap(volatile ptr
                             : i128, oldVal
                             : i128, newVal
                             : i128)
            ->i128
        {
                return __sync_val_compare_and_swap(
                    reinterpret_cast<int64_t *>(ptr),
                    (int64_t)(oldVal),
                    (int64_t)(newVal));
        }
#else
        fn Os_compareAndSwap(volatile ptr
                             : i128, oldVal
                             : i128, newVal
                             : i128)
            ->i128
        {
                return __sync_val_compare_and_swap(
                    reinterpret_cast<int64_t *>(ptr),
                    reinterpret_cast<int64_t>(oldVal),
                    reinterpret_cast<int64_t>(newVal));
        }
#endif
}