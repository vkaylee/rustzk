# Deployment Guide

## Installation
Add `rustzk` to your `Cargo.toml`:
```toml
[dependencies]
rustzk = { path = "../path/to/rustzk" }
```

## Configuration
Ensure your ZK device is reachable on the network and the default port `4370` is open.

## Network Troubleshooting
- **UDP**: Most legacy devices use UDP. Ensure no firewalls are blocking port 4370.
- **TCP**: Recommended for newer devices and stable data transfer of large logs.
