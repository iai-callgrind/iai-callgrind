# I'm getting the error Sentinel ... not found

You've most likely disabled creating debug symbols in your cargo `bench`
profile. This can originate in an option you've added to the `release` profile
since the `bench` profile inherits the `release` profile. For example, if you've
added `strip = true` to your `release` profile which is perfectly fine, you need
to disable this option in your `bench` profile to be able to run Iai-Callgrind
benchmarks.

See also the [Debug Symbols](../installation/prerequisites.md#debug-symbols)
section in Installation/Prerequisites.
