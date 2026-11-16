/* Hand-written replacement for the jconfig.h that libjpeg-turbo's CMake build
 * generates from src/src/jconfig.h.in. DCMfx builds libjpeg-turbo with cc-rs
 * rather than CMake, so the configuration is fixed here.
 *
 * SIMD extensions are enabled by build.rs defining DCMFX_LIBJPEG_TURBO_SIMD on
 * targets where they are supported.
 */

#define JPEG_LIB_VERSION  62

#define LIBJPEG_TURBO_VERSION  3.2.1

#define LIBJPEG_TURBO_VERSION_NUMBER  3002001

#define C_ARITH_CODING_SUPPORTED 1

#define D_ARITH_CODING_SUPPORTED 1

#define MEM_SRCDST_SUPPORTED  1

#ifdef DCMFX_LIBJPEG_TURBO_SIMD
#define WITH_SIMD 1
#endif

#ifndef BITS_IN_JSAMPLE
#define BITS_IN_JSAMPLE  8
#endif

#ifdef _WIN32

#undef RIGHT_SHIFT_IS_UNSIGNED

#ifndef __RPCNDR_H__            /* don't conflict if rpcndr.h already read */
typedef unsigned char boolean;
#endif
#define HAVE_BOOLEAN            /* prevent jmorecfg.h from redefining it */

#if !(defined(_BASETSD_H_) || defined(_BASETSD_H))   /* don't conflict if basetsd.h already read */
typedef short INT16;
typedef signed int INT32;
#endif
#define XMD_H                   /* prevent jmorecfg.h from redefining it */

#endif
