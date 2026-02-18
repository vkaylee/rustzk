use rustzk::ZK;
use rustzk::constants::*;
use rustzk::protocol::{ZKPacket, TCPWrapper};
use std::net::TcpListener;
use std::io::{Read, Write};
use std::thread;

#[test]
fn test_mock_tcp_connect() {
    // 1. Start a mock ZK server on a random port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // 2. Spawn server thread
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        
        // Receive the CMD_CONNECT packet
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).unwrap();
        
        let mut rdr = std::io::Cursor::new(&header);
        let _m1 = byteorder::ReadBytesExt::read_u16::<byteorder::LittleEndian>(&mut rdr).unwrap();
        let _m2 = byteorder::ReadBytesExt::read_u16::<byteorder::LittleEndian>(&mut rdr).unwrap();
        let length = byteorder::ReadBytesExt::read_u32::<byteorder::LittleEndian>(&mut rdr).unwrap() as usize;

        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).unwrap();
        
        let packet = ZKPacket::from_bytes(&body).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        // Send back ACK_OK
        // Use a fake session ID 1234
        let res_packet = ZKPacket::new(CMD_ACK_OK, 1234, packet.reply_id, vec![]);
        let res_bytes = res_packet.to_bytes();
        let wrapped = TCPWrapper::wrap(&res_bytes);
        stream.write_all(&wrapped).unwrap();
    });

    // 3. Run client
    let mut zk = ZK::new("127.0.0.1", port);
    let result = zk.connect(true);

    assert!(result.is_ok(), "Client failed to connect to mock server: {:?}", result.err());
    assert!(zk.is_connected);
    
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_mock_udp_connect() {
    use std::net::UdpSocket;
    
    let server_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP mock");
    let addr = server_socket.local_addr().unwrap();
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let (len, client_addr) = server_socket.recv_from(&mut buf).expect("Failed to receive UDP");
        
        let packet = ZKPacket::from_bytes(&buf[..len]).unwrap();
        assert_eq!(packet.command, CMD_CONNECT);

        // Send back ACK_OK
        let res_packet = ZKPacket::new(CMD_ACK_OK, 5678, packet.reply_id, vec![]);
        server_socket.send_to(&res_packet.to_bytes(), client_addr).unwrap();
    });

    let mut zk = ZK::new("127.0.0.1", port);
    let result = zk.connect(false); // UDP

    assert!(result.is_ok(), "UDP Client failed: {:?}", result.err());
    assert!(zk.is_connected);

    server_handle.join().unwrap();
}
