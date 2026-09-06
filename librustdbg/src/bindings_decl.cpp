#include <array>
#include <cstddef>
#include <cstring>

extern "C" {
std::array<std::byte, 16> double_to_bytes(double value) {

  std::array<std::byte, 16> ret;
  memset(&ret, 0, sizeof(std::array<std::byte, 16>));
  long double v = value;

	// thx
	// https://stackoverflow.com/questions/79421066/c-long-double-128-bit-precision
  memcpy(&ret, &v, 80/8);

  return ret;
}
}
