// SmearedPerlinNoiseMixin.modifyCoordinate2 and .noWrap: the block X/Z coordinate goes through
// adjustCoordinate and `coordinate * xzScale` is no longer wrapped.
fn smeared_add_to_volume(
    noise: &SmearedPerlinNoise,
    buffer: &mut [f32],
    volume: DensityVolume,
    xz_scale: f64,
    y_scale: f64,
    amplitude: f32,
) -> Result<(), DensityError> {
    port_classicfarlands_noise::add_to_volume(noise, buffer, volume, xz_scale, y_scale, amplitude)
}
