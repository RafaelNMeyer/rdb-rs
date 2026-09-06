#include <array>
#include <cstddef>
#include <cstring>

extern "C" {
std::array<std::byte, 16> double_to_bytes(double value) {
  std::array<std::byte, 16> ret;
  long double v = static_cast<long double>(value);
  memcpy(&ret, &v, sizeof(long double));
  return ret;
}
}
