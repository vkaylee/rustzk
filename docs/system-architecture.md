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

## Protocol Handling
The library handles the legacy ZK protocol which uses a fixed 8-byte header followed by a payload and a checksum. For TCP, an additional 8-byte wrapper (Magic + Length) is prepended.
