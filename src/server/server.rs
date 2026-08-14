use crate::config::config::{AppConfig, AudioConfig};
use crate::dto::request::audio::AudioData;
use crate::dto::response::angle::AngleData;
use crate::services::socket::SocketService;
use crate::services::spectral_vad::SpectralVAD;
use crate::utils::calculate_angle::calculate_angle;
use anyhow::Result;
use env_logger::Builder;
use log::info;
use std::net::SocketAddr;

pub fn run(cfg: &AppConfig) -> Result<()> {
    Builder::new().filter_level(cfg.log_level()).init();

    let server = SocketService::bind(cfg.addr())?;
    let mut vad1 = SpectralVAD::new(cfg.vad.clone());
    let mut vad2 = SpectralVAD::new(cfg.vad.clone());

    info!("Сервер слушает на {}", cfg.addr());

    let mut recv_buf = cfg.recv_buf();
    let mut send_buf = cfg.send_buf();

    loop {
        let (received_audio, src_addr) = server.recv_from::<AudioData>(&mut recv_buf)?;
        process_packet(
            &mut vad1,
            &mut vad2,
            &received_audio,
            &server,
            src_addr,
            &cfg.audio,
            &mut send_buf,
        )?;
    }
}

fn process_packet(
    vad1: &mut SpectralVAD,
    vad2: &mut SpectralVAD,
    audio_dto: &AudioData,
    server: &SocketService,
    src_addr: SocketAddr,
    audio_cfg: &AudioConfig,
    buf: &mut Vec<u8>,
) -> Result<()> {
    info!("Сервер получил AudioData от {}", src_addr);

    let (has_speech1, has_speech2) = rayon::join(
        || vad1.detect_speech(&audio_dto.mic1, audio_cfg.sample_rate),
        || vad2.detect_speech(&audio_dto.mic2, audio_cfg.sample_rate),
    );

    if !(has_speech1 || has_speech2) {
        info!("Речи нет, пропускаем");
        return Ok(());
    }

    info!(
        "Речь обнаружена (mic1: {}, mic2: {})",
        has_speech1, has_speech2
    );
    let angle = calculate_angle(
        &audio_dto.mic1,
        &audio_dto.mic2,
        audio_cfg.sample_rate,
        audio_cfg.mic_distance,
    );

    let angle_data = AngleData { angle };
    server.send_to(&angle_data, src_addr, buf)?;
    info!("Сервер отправил AngleData: {:?}", angle_data);
    Ok(())
}
