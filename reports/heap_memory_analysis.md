# 🧠 Heap Memory Analysis — rustzk

## Scope

Analyzed heap allocation patterns across:
- [lib.rs](file:///home/elt1541/lee/rustzk/src/lib.rs) — main library (1661 lines)
- [protocol.rs](file:///home/elt1541/lee/rustzk/src/protocol.rs) — ZKPacket, TCP wrapper
- [models.rs](file:///home/elt1541/lee/rustzk/src/models.rs) — User, Attendance, Finger structs

---

## ✅ Already Optimized

| Pattern | Where | Notes |
|---------|-------|-------|
| `Cow<[u8]>` payload | `ZKPacket` | Avoids cloning packet data |
| `from_bytes_owned` with `split_off` | `protocol.rs:257` | Efficient zero-copy parsing |
| `with_capacity` in buffers | `send_command`, `read_with_buffer` | Pre-allocated sizes |
| `extend_from_slice` chunking | `receive_chunk_into` | Avoids per-byte copies |

---

## ⚠️ Optimization Opportunities

### 1. `Vec::new()` for Empty Payloads — ~15 Occurrences (HIGH IMPACT)

```rust
// Current — allocates a Vec on heap EVERY call
self.send_command(CMD_CONNECT, Vec::new());
self.send_command(CMD_GET_FREE_SIZES, Vec::new());
self.send_command(CMD_EXIT, Vec::new());
self.send_command(CMD_FREE_DATA, Vec::new());
// ... ~15 more
```

**Fix**: Change `send_command` signature to accept `&[u8]` instead of `Vec<u8>`:
```rust
// Before:
pub(crate) fn send_command(&mut self, command: u16, payload: Vec<u8>) -> ...
// After:
pub(crate) fn send_command(&mut self, command: u16, payload: &[u8]) -> ...

// Callers change from:
self.send_command(CMD_CONNECT, Vec::new())
// To:
self.send_command(CMD_CONNECT, &[])  // zero-alloc!
```

**Impact**: Eliminates **15+ heap allocations per session** for commands with no payload.

---

### 2. UDP Read Buffer — Fixed 2KB Alloc Every Packet

```rust
// lib.rs:370 — inside read_packet()
let mut buf = vec![0u8; 2048];  // alloc every packet read!
```

**Fix**: Use a reusable buffer in the `ZK` struct:
```rust
pub struct ZK {
    // ...
    udp_buf: Vec<u8>,  // Reusable 2KB buffer
}

// In read_packet:
self.udp_buf.resize(2048, 0);
let len = socket.recv(&mut self.udp_buf)?;
```

**Impact**: Eliminates **1 heap alloc per UDP packet** (critical for attendance log reads with 100s of packets).

---

### 3. `get_users()` / `get_attendance()` — No Pre-Allocation for Result Vec

```rust
// lib.rs:741
let mut users = Vec::new();  // No idea how many users

// lib.rs:864
let mut attendances = Vec::new();  // Could be 10,000+ records
```

**Fix**: Pre-allocate based on known count from `read_sizes()`:
```rust
let mut users = Vec::with_capacity(self.users as usize);
let mut attendances = Vec::with_capacity(self.records as usize);
```

**Impact**: Eliminates **repeated Vec resizing** for large datasets. A device with 5,000 attendance records would cause ~12 reallocations without pre-alloc.

---

### 4. String Allocations in Parsing Loops (MEDIUM IMPACT)

```rust
// Inside get_users() loop — runs for EVERY user
group_id: group_id.to_string(),          // heap alloc
user_id: user_id.to_string(),            // heap alloc
password: String::from_utf8_lossy(...)
    .trim_matches('\0')
    .to_string(),                        // heap alloc

// Inside get_attendance() loop — runs for EVERY record
let user_id = user_id_num.to_string();   // heap alloc per record
```

**Assessment**: These are necessary — `User` and `Attendance` structs own their strings. Could be optimized with a string interner or `CompactString`, but **not worth the complexity** for this use case.

> [!NOTE]
> String allocations in parsing loops are inherent to the data model (structs own their strings). Optimizing these would require changing the public API, which is not recommended.

---

### 5. `read_with_buffer` Empty Payload — No Pre-Alloc

```rust
// lib.rs:619
let mut payload = Vec::new();  // Will write 11 bytes
payload.write_u8(1)?;
payload.write_u16::<LittleEndian>(command)?;
payload.write_u32::<LittleEndian>(fct as u32)?;
payload.write_u32::<LittleEndian>(ext)?;
```

**Fix**: Use `Vec::with_capacity(11)` or a stack array `[u8; 11]`.

**Impact**: Minor — only 1 alloc per `read_with_buffer` call.

---

### 6. `encoding` Field — Always "UTF-8"

```rust
// lib.rs:89
encoding: "UTF-8".to_string(),  // Heap alloc, never changes
```

**Fix**: Use `&'static str` instead of `String`:
```rust
pub encoding: &'static str,
// ...
encoding: "UTF-8",  // zero alloc
```

**Impact**: Minor — 1 alloc per `ZK::new()`, but shows intent better.

---

## 📊 Impact Summary

| Fix | Heap Allocs Saved | Effort | Priority |
|-----|-------------------|--------|----------|
| `send_command(&[u8])` | ~15/session | Medium | 🔴 High |
| Reusable UDP buffer | 1/packet | Low | 🔴 High |
| Pre-alloc result Vecs | ~12 resizes/fetch | Low | 🟡 Medium |
| `read_with_buffer` payload | 1/call | Low | 🟢 Low |
| `encoding: &'static str` | 1/new() | Low | 🟢 Low |
| String in loops | N/A | High | ⚪ Skip |

---

## 🏁 Recommendation

**Top 3 fixes** (best ROI):
1. **`send_command` accept `&[u8]`** — biggest impact, eliminates most unnecessary allocs
2. **Reusable UDP buffer** — important for high-volume attendance fetching
3. **Pre-allocate result Vecs** — easy win for large datasets
