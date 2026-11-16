/* Hand-written replacement for the jversion.h that libjpeg-turbo's CMake build
 * generates from src/src/jversion.h.in.
 */

#if JPEG_LIB_VERSION >= 80
#define JVERSION        "8d  15-Jan-2012"
#elif JPEG_LIB_VERSION >= 70
#define JVERSION        "7  27-Jun-2009"
#else
#define JVERSION        "6b  27-Mar-1998"
#endif

#define JCOPYRIGHT \
  "Copyright (C) 1991-2026 The libjpeg-turbo Project and many others"

#define JCOPYRIGHT_SHORT \
  "Copyright (C) 1991-2026 The libjpeg-turbo Project and many others"
