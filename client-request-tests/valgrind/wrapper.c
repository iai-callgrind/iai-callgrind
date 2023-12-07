#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

#include "valgrind/callgrind.h"
#include "valgrind/dhat.h"
// #include "valgrind/drd.h"
// #include "valgrind/helgrind.h"
#include "valgrind/memcheck.h"
#include "valgrind/valgrind.h"

void print_error(char *var) {
  fprintf(stderr,
          "valgrind client requests: ERROR: %s not defined! You may need to "
          "check your installed valgrind version having this client request "
          "available. The valgrind version of the valgrind.h header file is "
          "%d.%d. Exiting...\n",
          var, __VALGRIND_MAJOR__, __VALGRIND_MINOR__);
}

size_t running_on_valgrind() {
#ifdef RUNNING_ON_VALGRIND
  return RUNNING_ON_VALGRIND;
#else
  print_error("RUNNING_ON_VALGRIND");
  exit(-1);
#endif
}

// void valgrind_discard_translations(void *addr, size_t len) {
// #ifdef VALGRIND_DISCARD_TRANSLATIONS
//   VALGRIND_DISCARD_TRANSLATIONS(addr, len);
// #else
//   print_error("VALGRIND_DISCARD_TRANSLATIONS");
//   exit(-1);
// #endif
// }
//
// int valgrind_printf(char *message) { return VALGRIND_PRINTF("%s", message); }
