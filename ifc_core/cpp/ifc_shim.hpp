#pragma once

#ifndef NOMINMAX
#  define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#  define WIN32_LEAN_AND_MEAN
#endif
#ifdef Status
#  undef Status
#endif
#ifdef STATUS
#  undef STATUS
#endif

#ifndef IFCOPENSHELL_HAVE_ROCKSDB
#  define IFCOPENSHELL_HAVE_ROCKSDB 0
#endif
#ifndef HAVE_ROCKSDB
#  define HAVE_ROCKSDB 0
#endif

#include "ifcparse/IfcFile.h"