// SmearedPerlinNoiseMixin.modifyCoordinate, for the 2D entry point (it wrapped X and Z before
// calling get3; get3 now does the adjusting).
fn smeared_get2(noise: &SmearedPerlinNoise, x: f64, z: f64) -> f32 {
    port_classicfarlands_noise::get2(noise, x, z)
}
