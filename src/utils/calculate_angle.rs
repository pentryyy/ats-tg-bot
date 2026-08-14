/// Скорость звука в воздухе при 20°C (м/с)
const SPEED_OF_SOUND: f32 = 343.0;

/// Вычисляет угол прихода звука (в градусах) по двум сигналам.
///
/// # Аргументы
/// * `mic1`, `mic2` – векторы отсчётов (i16) с одинаковой частотой дискретизации
/// * `sample_rate` – частота дискретизации (Гц)
/// * `mic_distance` – расстояние между микрофонами (метры)
///
/// # Возвращает
/// Угол в градусах от -90 до +90 (отрицательный – звук слева, положительный – справа).
pub fn calculate_angle(mic1: &[i16], mic2: &[i16], sample_rate: u32, mic_distance: f32) -> f32 {
    let signal1: Vec<f32> = mic1.iter().map(|&x| x as f32).collect();
    let signal2: Vec<f32> = mic2.iter().map(|&x| x as f32).collect();

    let max_lag = (mic_distance * sample_rate as f32 / SPEED_OF_SOUND) as usize + 1;
    let n = signal1.len();
    let mut correlation = Vec::with_capacity(2 * max_lag + 1);

    let max_lag_isize = max_lag as isize;
    for lag in -max_lag_isize..=max_lag_isize {
        let mut sum = 0.0;
        let start1 = if lag < 0 { -lag } else { 0 };
        let start2 = if lag > 0 { lag } else { 0 };
        let len = n as isize - start1.max(start2);
        if len > 0 {
            for i in 0..(len as usize) {
                let idx1 = (start1 + i as isize) as usize;
                let idx2 = (start2 + i as isize) as usize;
                sum += signal1[idx1] * signal2[idx2];
            }
            correlation.push(sum);
        } else {
            correlation.push(0.0);
        }
    }

    let (max_index, _) = correlation
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();
    let lag_samples = max_index as isize - max_lag_isize;

    let tau = lag_samples as f32 / sample_rate as f32;
    let ratio = (tau * SPEED_OF_SOUND / mic_distance).clamp(-1.0, 1.0);
    ratio.asin().to_degrees()
}
