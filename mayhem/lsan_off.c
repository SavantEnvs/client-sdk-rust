/* Disables LeakSanitizer at build time (ASan stays on). build.sh compiles this with -gdwarf-3 and
 * links it ahead of every other object, which also makes it the first DWARF compile unit. */
int __lsan_is_turned_off(void) { return 1; }
