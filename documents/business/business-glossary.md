# Glossary: rustzk

## A
- **ACK_OK:** A protocol response message (code 2000) indicating the command was received and processed successfully.
- **Attendance Log:** A single record of an employee's check-in or check-out event.
- **ATTLOG:** Short for "Attendance Log", the commmand used to retrieve these records (`CMD_ATTLOG_RRQ`).

## B
- **Biometric Data:** Biological identifiers used for authentication, such as fingerprints or facial geometry.
- **Buffer:** A temporary storage area in memory used to hold data packets before processing.

## C
- **Checksum:** A calculated value used to verify the integrity of a transmitted data packet.
- **CMD:** Short for "Command", an instruction sent from the client to the device.
- **Connect:** The process of establishing a TCP or UDP session with the device.

## D
- **Device:** Specifically refers to ZKTeco time and attendance terminals.
- **Disconnect:** The process of properly closing a session and releasing resources.

## E
- **Endianness:** The order in which bytes are stored in memory (`rustzk` uses Little Endian).
- **Enrollment:** The process of registering a user's biometric data on the device.

## F
- **Face Template:** A mathematical representation of a user's facial features used for recognition.
- **Fingerprint Template:** A mathematical representation of a fingerprint used for matching.
- **Firmware:** The low-level software running on the ZKTeco device.

## G
- **GBK:** A character encoding used for Simplified Chinese characters, often found in older firmware versions.
- **Group ID:** A numeric identifier for grouping users (e.g., Department ID).

## H
- **Header:** The initial part of a data packet containing metadata like size, command ID, and checksum.

## I
- **ISO-8601:** An international standard for date and time representation (e.g., `2023-10-27T10:00:00+07:00`).

## L
- **Log:** See "Attendance Log".

## M
- **MAC Address:** A unique hardware identifier for the device's network interface.
- **Mock Server:** A simulated device used for testing without physical hardware.

## O
- **Offset:** The difference in time between UTC and the local time zone.
- **OpLog:** "Operation Log", records of administrative actions performed on the device.

## P
- **Packet:** A formatted unit of data transmitted over the network.
- **Platform:** The hardware architecture of the ZKTeco device (e.g., ZM100, ZMM200).
- **Polling:** Periodically checking the device for status updates or new data.
- **Privilege:** The level of access a user has (User vs Admin).
- **Protocol:** The set of rules governing communication (TCP/UDP, ZK Protocol).

## R
- **Record:** See "Attendance Log".
- **Reply ID:** A unique identifier linking a response to its original request.
- **Rust:** The systems programming language used to build `rustzk`.

## S
- **SDK:** Software Development Kit. `rustzk` is an alternative to the official ZK SDK.
- **Session ID:** A unique identifier for an active connection session.
- **Socket:** An endpoint for network communication.

## T
- **TCP:** Transmission Control Protocol, a reliable connection-oriented protocol.
- **Template:** See "Biometric Data".
- **Timezone:** A region of the globe that observes a uniform standard time.
- **Transport:** The underlying network mechanism (TCP or UDP).

## U
- **UDP:** User Datagram Protocol, a connectionless and faster protocol (but less reliable).
- **UID:** Internal User ID used by the device logic (different from the display User ID).
- **User ID:** The identifier displayed on the device screen (e.g., employee number).

## W
- **Workcode:** A numeric code entered by users to indicate a specific project or task (optional).

## Z
- **ZK Protocol:** The proprietary communication protocol used by ZKTeco devices.
