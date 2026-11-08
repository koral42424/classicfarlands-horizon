use super::{STACK_Y, SmearedPerlinNoise, YSample, add_four, compute_fudge_y, smoothstep, wrap};
use crate::{
    geometry::floor_i32,
    noise::{DensityError, DensityVolume, port_classicfarlands_state::adjust_coordinate},
};

// ---- Classic (double precision) Far Lands ---------------------------------------------------------
//
// Past 2^31 in noise space the cell of an octave stops moving (the int conversion saturates) while the
// position inside it keeps growing, so the fade polynomial reaches 1e45 and more. Classic Minecraft did
// this sum in doubles, so the terrain there is a huge number whose *sign* draws the Far Lands. Horizon's
// noise adds and fades in f32 (max 3.4e38) and its buffers are f32: past a few thousand blocks beyond
// the edge every sample is infinity or NaN, which is the smooth grey wall instead of the classic
// stripes. Samples whose X or Z cell has saturated are therefore computed here in f64, and their size
// is brought down to a finite value that still dwarfs ordinary noise (about 1e12 against about 1),
// keeping the sign and the proportions between the noises of one position.

/// Where a position stops being an ordinary cell: the int conversion of its floor saturates.
fn out_of_cell_range(shifted: f64) -> bool {
    !(shifted >= -2147483648.0 && shifted < 2147483647.0)
}

/// Mth.floor as Java runs it: `(int) d` saturates, then `d < i ? i - 1 : i` wraps, so far below the
/// range the cell comes out as `i32::MAX`.
fn java_floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// How big a saturated sample is out of the cell (the fade of each saturated axis grows with the fifth
/// power of its distance, the corner dot products with the first).
fn growth(relative_x: f64, relative_z: f64) -> f64 {
    let ax = relative_x.abs().max(1.0);
    let az = relative_z.abs().max(1.0);
    ax.powi(5) * az.powi(5) * ax.max(az)
}

/// The size a saturated sample is brought down to.
const CLASSIC_TARGET: f64 = 1.0e12;

/// The factor that brings a sample's size down to `CLASSIC_TARGET` (1.0 while it is smaller). Both
/// limit noises of a position share it, so their proportions, which decide the sign of the blend, stay.
fn classic_scale(relative_x: f64, relative_z: f64) -> f64 {
    let size = growth(relative_x, relative_z);
    if size > CLASSIC_TARGET {
        CLASSIC_TARGET / size
    } else {
        1.0
    }
}

fn fade64(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp64(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// `PerlinNoise::sample` in f64: the eight corner dot products and the trilinear blend of the faded position.
fn sample64(gradients: [[f32; 3]; 8], point: [f64; 3], interpolation_y: f64) -> f64 {
    let values: [f64; 8] = std::array::from_fn(|i| {
        let g = gradients[i];
        f64::from(g[0]) * (point[0] - (i & 1) as f64)
            + f64::from(g[1]) * (point[1] - ((i >> 1) & 1) as f64)
            + f64::from(g[2]) * (point[2] - ((i >> 2) & 1) as f64)
    });
    let fade_x = fade64(point[0]);
    let fade_y = fade64(interpolation_y);
    let fade_z = fade64(point[2]);
    let plane = |from: usize| {
        lerp64(
            fade_y,
            lerp64(fade_x, values[from], values[from + 1]),
            lerp64(fade_x, values[from + 2], values[from + 3]),
        )
    };
    lerp64(fade_z, plane(0), plane(4))
}

/// One octave's sample at a position whose X or Z cell has saturated, brought down to a finite f32.
fn classic_get3(noise: &SmearedPerlinNoise, shifted: [f64; 3], y: f64) -> f32 {
    let cell = [
        java_floor(shifted[0]),
        floor_i32(shifted[1]),
        java_floor(shifted[2]),
    ];
    let relative: [f64; 3] = std::array::from_fn(|i| shifted[i] - f64::from(cell[i]));
    let fudge = compute_fudge_y(y, relative[1], noise.fudge_y_scale);
    let value = sample64(
        noise.base.gradients(cell),
        [relative[0], relative[1] - fudge, relative[2]],
        relative[1],
    );
    (value * classic_scale(relative[0], relative[2])) as f32
}

/// SmearedPerlinNoise.get with ClassicFarlandsMixin's changes: X and Z go through
/// `adjust_coordinate` and are not wrapped; Y is wrapped as before.
pub fn get3(noise: &SmearedPerlinNoise, x: f64, y: f64, z: f64) -> f32 {
    let offsets = noise.base.table.offsets;
    let shifted = [
        adjust_coordinate(x) + offsets[0],
        wrap(y) + offsets[1],
        adjust_coordinate(z) + offsets[2],
    ];
    if out_of_cell_range(shifted[0]) || out_of_cell_range(shifted[2]) {
        return classic_get3(noise, shifted, y);
    }
    let cell = shifted.map(floor_i32);
    let relative: [f64; 3] = std::array::from_fn(|i| shifted[i] - f64::from(cell[i]));
    let fudge = compute_fudge_y(y, relative[1], noise.fudge_y_scale);
    noise.base.sample(
        cell,
        [
            relative[0] as f32,
            (relative[1] - fudge) as f32,
            relative[2] as f32,
        ],
        relative[1] as f32,
    )
}

pub fn get2(noise: &SmearedPerlinNoise, x: f64, z: f64) -> f32 {
    get3(noise, x, 0.0, z)
}

/// SmearedPerlinNoise.addToVolume with ClassicFarlandsMixin's changes: the X and Z block
/// coordinates go through `adjust_coordinate`, and `coordinate * xz_scale` is not wrapped.
/// The rest is `PerlinNoise::add_volume::<true>` with this noise's fudge, unchanged.
pub fn add_to_volume(
    noise: &SmearedPerlinNoise,
    buffer: &mut [f32],
    volume: DensityVolume,
    xz_scale: f64,
    y_scale: f64,
    amplitude: f32,
) -> Result<(), DensityError> {
    let base = &noise.base;
    let fudge = noise.fudge_y_scale;
    if buffer.len() < volume.size() {
        return Err(DensityError::BufferTooShort);
    }
    let [size_x, size_y, size_z] = volume.sizes();
    let mut stack = [YSample::default(); STACK_Y];
    let mut heap = Vec::new();
    let ys = if size_y <= STACK_Y {
        &mut stack[..size_y]
    } else {
        heap.resize(size_y, YSample::default());
        &mut heap[..]
    };
    for (y, sample) in ys.iter_mut().enumerate() {
        let input_y = f64::from(volume.coordinate(1, y)) * y_scale;
        let shifted_y = wrap(input_y) + base.table.offsets[1];
        let cell = floor_i32(shifted_y);
        let dy = shifted_y - f64::from(cell);
        *sample = YSample {
            cell,
            fade: smoothstep(dy as f32),
            offset: (dy - compute_fudge_y(input_y, dy, fudge)) as f32,
        };
    }
    let ys = &*ys;
    let mut index = 0;
    for z in 0..size_z {
        let shifted_z = adjust_coordinate(f64::from(volume.coordinate(2, z))) * xz_scale
            + base.table.offsets[2];
        let cell_z = floor_i32(shifted_z);
        let dz = (shifted_z - f64::from(cell_z)) as f32;
        let fade_z = smoothstep(dz);
        for x in 0..size_x {
            let shifted_x = adjust_coordinate(f64::from(volume.coordinate(0, x))) * xz_scale
                + base.table.offsets[0];
            if out_of_cell_range(shifted_x) || out_of_cell_range(shifted_z) {
                // The column is in the Far Lands proper: every sample in f64 (see classic_get3).
                let cell_x = java_floor(shifted_x);
                let cell_z = java_floor(shifted_z);
                let relative_x = shifted_x - f64::from(cell_x);
                let relative_z = shifted_z - f64::from(cell_z);
                let scale = classic_scale(relative_x, relative_z);
                let p = base.table.permute(cell_x);
                let q = base.table.permute(cell_x.wrapping_add(1));
                for y in 0..size_y {
                    let input_y = f64::from(volume.coordinate(1, y)) * y_scale;
                    let shifted_y = wrap(input_y) + base.table.offsets[1];
                    let cell_y = floor_i32(shifted_y);
                    let dy = shifted_y - f64::from(cell_y);
                    let offset_y = dy - compute_fudge_y(input_y, dy, fudge);
                    let value = sample64(
                        base.gradients_at(p, q, cell_y, cell_z),
                        [relative_x, offset_y, relative_z],
                        dy,
                    );
                    buffer[index + y] += amplitude * ((value * scale) as f32);
                }
                index += size_y;
                continue;
            }
            let cell_x = floor_i32(shifted_x);
            let dx = (shifted_x - f64::from(cell_x)) as f32;
            let fade_x = smoothstep(dx);
            let p = base.table.permute(cell_x);
            let q = base.table.permute(cell_x.wrapping_add(1));
            let mut previous_y = i32::MIN;
            let mut xz_dots = [0.0_f32; 8];
            let mut y_gradients = [0.0_f32; 8];
            let column = &mut buffer[index..index + size_y];
            let mut y = 0;
            while y < size_y {
                let sample = &ys[y];
                if previous_y != sample.cell {
                    let gradients = base.gradients_at(p, q, sample.cell, cell_z);
                    for i in 0..8 {
                        xz_dots[i] = gradients[i][0] * (dx - (i & 1) as f32)
                            + gradients[i][2] * (dz - ((i >> 2) & 1) as f32);
                        y_gradients[i] = gradients[i][1];
                    }
                    previous_y = sample.cell;
                }
                if y + 4 <= size_y && ys[y + 1..y + 4].iter().all(|s| s.cell == sample.cell) {
                    add_four(
                        &mut column[y..y + 4],
                        &ys[y..y + 4],
                        xz_dots,
                        y_gradients,
                        fade_x,
                        fade_z,
                        amplitude,
                    );
                    y += 4;
                    continue;
                }
                let out = &mut column[y];
                let dy = sample.offset;
                let values = std::array::from_fn(|i| {
                    xz_dots[i] + y_gradients[i] * (dy - ((i >> 1) & 1) as f32)
                });
                *out += amplitude * super::lerp3([fade_x, sample.fade, fade_z], values);
                y += 1;
            }
            index += size_y;
        }
    }
    Ok(())
}

#[cfg(test)]
mod classic_tests {
    use super::*;

    #[test]
    fn java_floor_wraps_far_below_the_range_and_saturates_above() {
        assert_eq!(java_floor(2.5), 2);
        assert_eq!(java_floor(-2.5), -3);
        assert_eq!(java_floor(5.0e9), i32::MAX);
        assert_eq!(java_floor(-5.0e9), i32::MAX);
        assert!(out_of_cell_range(5.0e9) && out_of_cell_range(-5.0e9));
        assert!(!out_of_cell_range(2147483646.5) && !out_of_cell_range(-2147483648.0));
    }

    #[test]
    fn scale_leaves_ordinary_positions_alone_and_caps_the_far_ones() {
        assert_eq!(classic_scale(0.25, 0.75), 1.0);
        assert_eq!(classic_scale(50.0, 0.5), 1.0); // 50^6 = 1.6e10, below the target
        let far = classic_scale(2.0e9, 0.5);
        assert!(far > 0.0 && far < 1.0e-40);
        // The sample it scales (about growth(..) in size) lands on the target, in f64.
        assert!((far * growth(2.0e9, 0.5) / CLASSIC_TARGET - 1.0).abs() < 1e-12);
        let corner = classic_scale(3.0e9, -4.0e9);
        assert!(corner.is_finite() && corner > 0.0);
        // The one scale is continuous across the threshold.
        let just_under = growth(100.0, 0.0);
        assert!((just_under - CLASSIC_TARGET).abs() < 1.0);
    }

    #[test]
    fn sample64_blends_the_corners() {
        // Gradients along +x everywhere: corner value = x - (i & 1).
        let gradients = [[1.0_f32, 0.0, 0.0]; 8];
        let at = |x: f64| sample64(gradients, [x, 0.3, 0.6], 0.3);
        assert!((at(0.0) - 0.0).abs() < 1e-12);
        // At x = 1 the x fade is 1, so the value is the far corner's: 1 - 1 = 0.
        assert!(at(1.0).abs() < 1e-12);
        // A saturated position still gives a finite number where f32 would not.
        assert!(at(2.0e9).is_finite());
        assert!((fade64(0.0)).abs() < 1e-15 && (fade64(1.0) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn the_sign_of_a_huge_sample_does_not_depend_on_how_far_out_it_is() {
        // Two corners pull opposite ways in x: far out the blend is dominated by the x fade, so
        // moving 1000 blocks further changes the size, not the side.
        let mut gradients = [[0.0_f32; 3]; 8];
        for i in 0..8 {
            gradients[i] = if i & 1 == 0 { [1.0, 0.0, 0.0] } else { [-1.0, 0.0, 0.0] };
        }
        let near = sample64(gradients, [1.0e6, 0.4, 0.4], 0.4);
        let farther = sample64(gradients, [1.0e6 + 1.0e3, 0.4, 0.4], 0.4);
        assert!(near.is_finite() && farther.is_finite());
        assert_eq!(near.is_sign_positive(), farther.is_sign_positive());
    }
}
