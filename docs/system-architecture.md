# System Architecture

## Architecture Diagram
```ascii
┌────────────────────────────────┐
│           User Code            │
└───────────────┬────────────────┘
                │
        ┌───────▼───────┐
        │      ZK       │ (src/lib.rs)
        └───────┬───────┘
                │
        ┌───────▼───────┐
        │   Protocol    │ (src/protocol.rs)
        └───────┬───────┘
                │
    ┌───────────┴───────────┐
    │                       │
┌───▼───┐               ┌───▼───┐
│  TCP  │               │  UDP  │
└───┬───┘               └───┬───┘
    │                       │
    └───────────┬───────────┘
                │
        ┌───────▼───────┐
        │  ZK Device    │
        └───────────────┘
```

## Memory Efficiency & Zero-Copy
The library is designed for high-performance concurrent environments.
- **Zero-Copy Packets**: `ZKPacket` uses `Cow<'a, [u8]>` to wrap payload data. When reading from the network, packets borrow directly from the read buffer where possible.
- **In-Place Deserialization**: `read_packet` minimizes allocations by reading headers and bodies into target structures without intermediate copies.
- **Buffer Pre-allocation**: Chunked data transfers (e.g., thousands of attendance records) use the `_into` pattern to stream data directly into pre-reserved vectors, avoiding repeated `realloc` calls.
- **Single-Buffer Serialization**: Outgoing packets are wrapped and serialized into a single contiguous buffer to minimize system call overhead.
