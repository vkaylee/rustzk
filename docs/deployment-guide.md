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

## Publishing to Crates.io
This project uses GitHub Actions to automate the publishing process.

### Configuration
1.  Obtain an API token from [crates.io](https://crates.io/settings/tokens).
2.  Add the token to your GitHub repository secrets as `CRATES_IO_TOKEN`.

### Release Workflow
To publish a new version:
1.  Update the version in `Cargo.toml`.
2.  Commit and push the changes to `master`.
3.  Create and push a new git tag starting with `v` (e.g., `v0.1.0`):
    ```bash
    git tag v0.1.0
    git push origin v0.1.0
    ```
The GitHub Action will automatically run the CI checks and publish the crate if they pass.
