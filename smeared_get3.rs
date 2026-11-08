// SmearedPerlinNoiseMixin.modifyCoordinate: `wrap(x)` and `wrap(z)` become adjustCoordinate.
fn smeared_get3(noise: &SmearedPerlinNoise, x: f64, y: f64, z: f64) -> f32 {
    port_classicfarlands_noise::get3(noise, x, y, z)
}
