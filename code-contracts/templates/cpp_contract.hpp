#pragma once

#include <cstdint>
#include <string>
#include <string_view>
#include <stdexcept>

namespace storage {

class KeyNotFoundError : public std::runtime_error {
    using std::runtime_error::runtime_error;
};

class StorageFullError : public std::runtime_error {
    using std::runtime_error::runtime_error;
};

/**
 * @brief Memory-mapped ring buffer cache for low-latency metric ingestion.
 *
 * @invariant capacity > 0
 * @invariant size() <= capacity()
 * @invariant Memory pages remain pinned throughout object lifetime.
 */
class RingBufferCache {
public:
    /**
     * @brief Constructs a bounded cache with pre-allocated memory-mapped capacity.
     *
     * @pre max_records > 0
     * @post capacity() == max_records
     * @post size() == 0
     *
     * @param max_records Maximum entries permissible before rotation.
     * @throw std::bad_alloc If pre-allocation of physical pages fails.
     */
    explicit RingBufferCache(std::size_t max_records);

    /**
     * @brief Inserts or updates an entry in the bounded ring buffer.
     *
     * @pre !key.empty()
     * @pre payload_size > 0 && payload_size <= 65536
     * @post size() <= capacity()
     *
     * @param key Unique null-terminated domain lookup key.
     * @param payload_bytes Pointer to raw contiguous bytes.
     * @param payload_size Byte length of payload.
     *
     * @return uint64_t Monotonically increasing sequence offset.
     *
     * @throw std::invalid_argument If key is empty or payload_size exceeds limit.
     * @throw StorageFullError If buffer is unevictable and full.
     */
    uint64_t put(std::string_view key, const uint8_t* payload_bytes, std::size_t payload_size);

    [[nodiscard]] std::size_t size() const noexcept;
    [[nodiscard]] std::size_t capacity() const noexcept;
};

} // namespace storage
