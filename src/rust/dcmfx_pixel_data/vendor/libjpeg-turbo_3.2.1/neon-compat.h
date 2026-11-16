/* Hand-written replacement for the neon-compat.h that libjpeg-turbo's CMake
 * build generates from src/simd/arm/neon-compat.h.in. This is only used on
 * AArch64, where all supported compilers provide these intrinsics.
 */

#define HAVE_VLD1_S16_X3
#define HAVE_VLD1_U16_X2
#if defined(__clang__) || (defined(__GNUC__) && __GNUC__ >= 9)
#define HAVE_VLD1Q_U8_X4
#endif

#include <arm_neon.h>

#if defined(_MSC_VER) && !defined(__clang__)
#define BUILTIN_CLZ(x)  _CountLeadingZeros(x)
#define BUILTIN_CLZLL(x)  _CountLeadingZeros64(x)
#define BUILTIN_BSWAP64(x)  _byteswap_uint64(x)
#elif defined(__clang__) || defined(__GNUC__)
#define BUILTIN_CLZ(x)  __builtin_clz(x)
#define BUILTIN_CLZLL(x)  __builtin_clzll(x)
#define BUILTIN_BSWAP64(x)  __builtin_bswap64(x)
#else
#error "Unknown compiler"
#endif

#if defined(__clang__)
#pragma clang diagnostic ignored "-Wdeclaration-after-statement"
#pragma clang diagnostic ignored "-Wc99-extensions"
#elif defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wdeclaration-after-statement"
#pragma GCC diagnostic ignored "-Wpedantic"
#endif
