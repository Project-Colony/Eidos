#include "NifUtil.hpp"

#include <cstdint>
#include <deque>
#include <numeric>
#include <stdexcept>
#include <vector>

template<typename Container>
void checkErase(size_t count, const std::vector<uint16_t>& indices) {
    Container values(count);
    std::iota(values.begin(), values.end(), 0u);
    Container expected = values;
    for (auto it = indices.rbegin(); it != indices.rend(); ++it)
        expected.erase(expected.begin() + *it);

    nifly::EraseVectorIndices(values, indices);
    // Keep the check active in Release builds, which define NDEBUG.
    if (values != expected)
        throw std::runtime_error("EraseVectorIndices lost or reordered retained elements");
}

template<typename Container>
void checkIndexBoundary() {
    checkErase<Container>(0, {});
    for (size_t count : {65535u, 65536u, 65537u}) {
        checkErase<Container>(count, {});
        checkErase<Container>(count, {0});
        checkErase<Container>(count, {65534});
        checkErase<Container>(count, {0, 32767, 65534});
    }
    // Starting immediately after UINT16_MAX must not wrap to zero either.
    checkErase<Container>(65536, {65535});
    checkErase<Container>(65537, {65535});
}

int main() {
    checkIndexBoundary<std::vector<unsigned>>();
    checkIndexBoundary<std::deque<unsigned>>();
}
