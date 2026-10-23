import socket
import struct
import cv2
import numpy as np

UDP_IP = "0.0.0.0"
UDP_PORT = 5000

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
# Increase OS receive buffer to 4MB to prevent packet loss during bursts
sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 4 * 1024 * 1024)
sock.bind((UDP_IP, UDP_PORT))

print(f"Listening for ESP32-CAM UDP stream on port {UDP_PORT}...")

current_frame_id = -1
chunks = {}
expected_chunks = 0

while True:
    data, _ = sock.recvfrom(2048)
    if len(data) < 8:
        continue

    # Unpack 8-byte header: <IHH = uint32 (frame_id), uint16 (total_chunks), uint16 (chunk_idx)
    frame_id, total_chunks, chunk_idx = struct.unpack("<IHH", data[:8])
    payload = data[8:]

    # If a new frame arrives, reset state (automatically drops incomplete old frames for minimum latency)
    if frame_id != current_frame_id:
        current_frame_id = frame_id
        chunks = {}
        expected_chunks = total_chunks

    chunks[chunk_idx] = payload

    # Once all chunks for the current frame arrive, decode and display
    if len(chunks) == expected_chunks:
        jpeg_bytes = b"".join(chunks[i] for i in range(expected_chunks))
        chunks.clear()

        # Decode JPEG in memory
        np_arr = np.frombuffer(jpeg_bytes, dtype=np.uint8)
        frame = cv2.imdecode(np_arr, cv2.IMREAD_COLOR)

        if frame is not None:
            # --- RUN YOUR COMPUTER VISION / PROCESSING HERE ---
            cv2.imshow("ESP32-CAM Low-Latency Stream", frame)
            if cv2.waitKey(1) & 0xFF == ord('q'):
                break

cv2.destroyAllWindows()