/* Hand-written replacement for the jconfigint.h that libjpeg-turbo's CMake
 * build generates from src/src/jconfigint.h.in. DCMfx builds libjpeg-turbo
 * with cc-rs rather than CMake, so the configuration is determined here using
 * the preprocessor.
 */

#define BUILD  "dcmfx"

#if defined(_MSC_VER) && !defined(__clang__)
#define HIDDEN
#else
#define HIDDEN  __attribute__((visibility("hidden")))
#endif

#undef inline
#if defined(_MSC_VER) && !defined(__clang__)
#define INLINE  __forceinline
#define THREAD_LOCAL  __declspec(thread)
#else
#define INLINE  __inline__ __attribute__((always_inline))
#define THREAD_LOCAL  __thread
#endif

#define PACKAGE_NAME  "libjpeg-turbo"

#define VERSION  "3.2.1"

#if defined(_WIN64) || defined(__LP64__) || defined(__x86_64__) || \
    defined(__aarch64__)
#define SIZEOF_SIZE_T  8
#else
#define SIZEOF_SIZE_T  4
#endif

#if defined(__GNUC__) || defined(__clang__)
#define HAVE_BUILTIN_CTZL
#endif

#if defined(_MSC_VER)
#define HAVE_INTRIN_H
#endif

#if defined(_MSC_VER) && defined(HAVE_INTRIN_H)
#if (SIZEOF_SIZE_T == 8)
#define HAVE_BITSCANFORWARD64
#elif (SIZEOF_SIZE_T == 4)
#define HAVE_BITSCANFORWARD
#endif
#endif

#if defined(__has_attribute)
#if __has_attribute(fallthrough)
#define FALLTHROUGH  __attribute__((fallthrough));
#else
#define FALLTHROUGH
#endif
#else
#define FALLTHROUGH
#endif

#ifndef BITS_IN_JSAMPLE
#define BITS_IN_JSAMPLE  8      /* use 8 or 12 */
#endif

#undef C_ARITH_CODING_SUPPORTED
#undef D_ARITH_CODING_SUPPORTED
#undef WITH_SIMD

#if BITS_IN_JSAMPLE == 8

#define C_ARITH_CODING_SUPPORTED 1
#define D_ARITH_CODING_SUPPORTED 1

#ifdef DCMFX_LIBJPEG_TURBO_SIMD
#define WITH_SIMD 1
#define SIMD_ARCHITECTURE  ARM64
#else
#define SIMD_ARCHITECTURE  NONE
#endif

#endif
