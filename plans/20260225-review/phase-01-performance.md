# Phase 1: Network Performance Improvements

## 🟠 HIGH (Should Fix)

### Issue 1: Nagle's Algorithm (TCP_NODELAY) active on TCP Streams
- **File:** `src/lib.rs:178` (in `connect_tcp`)
- **Problem:** The device uses a command-response protocol transmitting very small packets (typically 8 to 40 bytes) over TCP. `TcpStream` in Rust leaves Nagle's algorithm enabled by default. This causes the OS to artificially delay sending small packets by up to 40-200ms while waiting for more data, drastically increasing latency when sending sequential commands or fetching multiple chunks.
- **Fix:** Disable Nagle's algorithm by setting `set_nodelay(true)` immediately after connecting.
```rust
// Before
let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
stream.set_read_timeout(Some(self.timeout))?;

// After
let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
stream.set_nodelay(true)?; // Disable Nagle's algorithm
stream.set_read_timeout(Some(self.timeout))?;
```

---

## 🟡 MEDIUM (Recommended)

### Issue 2: N+1 Syscalls on Packet Reads
- **File:** `src/lib.rs:346` (in `read_packet`)
- **Problem:** When reading a TCP packet, the code performs two sequential `stream.read_exact()` calls. Since `stream` is an unbuffered `TcpStream`, each `read_exact` corresponds directly to a `recv()` syscall. For a protocol with heavy back-and-forth like retrieving a 20,000-record attendance log in small chunks, this doubles the number of syscalls, increasing CPU overhead.
- **Fix:** While adding a full `BufReader` might complicate the `ZKTransport` enum, you can allocate a single buffer large enough to hold typical payloads (or peek the length) or just keep the current behavior but consider a slightly larger static read buffer for the header to minimize the second read if the packet is small enough. For now, replacing the two `read_exact` calls with a single read into a larger buffer if possible could be complex due to the `TCPWrapper` structure. Consider using `std::io::BufReader<TcpStream>` inside the `ZKTransport::Tcp` enum variant.

### Issue 3: Missing UDP Buffer Tuning
- **File:** `src/lib.rs:186` (in `connect_udp`)
- **Problem:** High-volume packet streams (like fetching large attendance logs over UDP) can overwhelm the default OS UDP receive buffer (`SO_RCVBUF`), causing packet loss and requiring retries or failing outright.
- **Fix:** Increase the UDP receive buffer size to a reasonable amount (e.g., 2MB) for `UdpSocket` using `socket.set_recv_buffer_size(2 * 1024 * 1024)` during connection if supported, or at least document that high-volume operations over UDP may experience packet loss if OS buffers are small.

---

## 🟢 LOW (Optional)

### Issue 4: Artificial Delay during Buffer Reads
- **File:** `src/lib.rs:669` (in `read_with_buffer`)
- **Problem:** The polling loop uses `std::thread::sleep(std::time::Duration::from_millis(10))` when receiving an empty response chunk. This adds a hard-coded 10ms artificial delay which could compound significantly if the device is slow to prepare a large attendance buffer.
- **Suggestion:** Consider an exponential backoff strategy instead of a fixed 10ms sleep, starting at 1ms up to 50ms, to improve responsiveness on fast devices while protecting against CPU spinning on slow devices.