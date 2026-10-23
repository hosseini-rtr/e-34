use anyhow::{Context, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_wifi_set_ps, link_patches, wifi_ps_type_t_WIFI_PS_NONE};
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration as WifiConfig, EspWifi,
};
use heapless::Vec;
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rusty_esp_image_core::esp_core::frame::Planes;
use rusty_esp_image_core::jpeg::find_eoi;
use rusty_esp_image_core::sensor::{FrameSize, Mode};
use rusty_esp_image_core::source::ImageSource;
use rusty_esp_image_esp::idf::IdfCamera;
use rusty_esp_image_esp::AI_THINKER_ESP32_CAM;

const WIFI_SSID: &str = "iot-home";
const WIFI_PASS: &str = "6fe4f300cf";

const TARGET_ADDR: &str = "192.168.1.10:5000"; // Computer IP and Port

const FRAME_SIZE: FrameSize = FrameSize::Vga;
const JPEG_QUALITY: u8 = 12;
const FRAME_BUFFERS: u8 = 2;
const FRAME_BUFFER_BYTES: usize = 64 * 1024;
const SETTLE_FRAMES: usize = 8;

const HEADER_SIZE: usize = 8;
const MAX_PAYLOAD: usize = 1400;

fn main() -> Result<()> {
    link_patches();
    EspLogger::initialize_default();

    // 1. Camera Init
    let mut mode = Mode::jpeg(FRAME_SIZE).context("create JPEG camera mode")?;
    mode.jpeg_quality = JPEG_QUALITY;

    let mut camera = IdfCamera::init(&AI_THINKER_ESP32_CAM, &mode, FRAME_BUFFERS)
        .context("initialize camera")?;
    camera.set_orientation(false, false)?;

    // 2. Wi-Fi Init
    let peripherals = Peripherals::take().context("take peripherals")?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;
    connect_wifi(&mut wifi)?;

    // Disable Wi-Fi power save for minimal transmission jitter
    unsafe {
        esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
    }

    // 3. UDP Socket Init
    let socket = UdpSocket::bind("0.0.0.0:0").context("bind socket")?;
    socket.connect(TARGET_ADDR).context("connect socket")?;

    let mut frame_buffer = vec![0u8; FRAME_BUFFER_BYTES];
    // Allocate the packet on the heap (Vec) rather than stack to prevent task stack overflows
    let mut tx_packet = vec![0u8; HEADER_SIZE + MAX_PAYLOAD];

    for _ in 0..SETTLE_FRAMES {
        let _ = camera.grab(&mut frame_buffer);
    }

    println!("STREAMING_START target={TARGET_ADDR}");

    let mut frame_id: u32 = 0;
    let mut stat_frames: u32 = 0;
    let mut started = Instant::now();

    loop {
        match camera.grab(&mut frame_buffer) {
            Ok(frame) => {
                if let Planes::Packed(jpeg) = frame.planes {
                    let jpeg_len = find_eoi(jpeg).unwrap_or(jpeg.len());
                    let jpeg_slice = &jpeg[..jpeg_len];
                    let total_chunks = ((jpeg_len + MAX_PAYLOAD - 1) / MAX_PAYLOAD) as u16;

                    for (chunk_idx, chunk) in jpeg_slice.chunks(MAX_PAYLOAD).enumerate() {
                        tx_packet[0..4].copy_from_slice(&frame_id.to_le_bytes());
                        tx_packet[4..6].copy_from_slice(&total_chunks.to_le_bytes());
                        tx_packet[6..8].copy_from_slice(&(chunk_idx as u16).to_le_bytes());
                        tx_packet[HEADER_SIZE..HEADER_SIZE + chunk.len()].copy_from_slice(chunk);

                        let _ = socket.send(&tx_packet[..HEADER_SIZE + chunk.len()]);
                    }

                    frame_id = frame_id.wrapping_add(1);
                    stat_frames += 1;

                    // Reduce log frequency: printing over serial every few frames introduces blocking latency
                    if stat_frames == 120 {
                        let elapsed = started.elapsed().as_secs_f32();
                        println!("FPS: {:.1} | Size: {} B", 120.0 / elapsed, jpeg_len);
                        stat_frames = 0;
                        started = Instant::now();
                    }
                }
            }
            Err(_) => {
                sleep(Duration::from_millis(1));
            }
        }
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    let mut auth_method = AuthMethod::WPA2Personal;
    if WIFI_PASS.is_empty() {
        auth_method = AuthMethod::None;
    }

    let wifi_configuration = WifiConfig::Client(ClientConfiguration {
        ssid: heapless::String::try_from(WIFI_SSID).unwrap(),
        password: heapless::String::try_from(WIFI_PASS).unwrap(),
        auth_method,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    println!("Wi-Fi radio started. Connecting to '{}'...", WIFI_SSID);

    wifi.connect()?;
    println!("Associated with AP, acquiring IP via DHCP...");

    wifi.wait_netif_up()?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wi-Fi connected! ESP32 IP: {}", ip_info.ip);

    Ok(())
}
