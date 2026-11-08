use std::sync::atomic::{AtomicI32, Ordering};

/// ClassicFarlands.BASE_DISTANCE: where the Far Lands are in the classic game.
pub const BASE_DISTANCE: i32 = 12550824;

/// The config's `distance` ("precise to within 4 blocks").
static DISTANCE: AtomicI32 = AtomicI32::new(BASE_DISTANCE);
/// ClassicFarlands.COORDINATE_DISTANCE: Java sets it when the server starts; the default config's value until then.
static COORDINATE_DISTANCE: AtomicI32 = AtomicI32::new(coordinate_distance_for(BASE_DISTANCE));

/// `4 * (distance / 4) - 2`, in Java's int arithmetic.
pub const fn coordinate_distance_for(distance: i32) -> i32 {
    4i32.wrapping_mul(distance / 4).wrapping_sub(2)
}

/// `4 * (distance / 4) - 3`: where the final density switches to the Far Lands one.
pub const fn switch_distance_for(distance: i32) -> i32 {
    4i32.wrapping_mul(distance / 4).wrapping_sub(3)
}

/// Sets the config's `distance` (the mod's `onInitialize` does it before any world loads).
pub fn set_distance(distance: i32) {
    DISTANCE.store(distance, Ordering::Relaxed);
    COORDINATE_DISTANCE.store(coordinate_distance_for(distance), Ordering::Relaxed);
}

pub fn distance() -> i32 {
    DISTANCE.load(Ordering::Relaxed)
}

pub fn coordinate_distance() -> i32 {
    COORDINATE_DISTANCE.load(Ordering::Relaxed)
}

pub fn switch_distance() -> i32 {
    switch_distance_for(distance())
}

/// ClassicFarlands.adjustCoordinate: pushes a noise coordinate out to where the classic Far Lands are.
pub fn adjust_coordinate(coordinate: f64) -> f64 {
    let limit = coordinate_distance();
    if coordinate > f64::from(limit) {
        (coordinate + f64::from(BASE_DISTANCE)) - f64::from(limit)
    } else if coordinate < f64::from(limit.wrapping_neg()) {
        (coordinate - f64::from(BASE_DISTANCE)) + f64::from(limit)
    } else {
        coordinate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values worked out by hand from the Java source:
    //   COORDINATE_DISTANCE = 4 * (distance / 4) - 2, switch distance = 4 * (distance / 4) - 3.
    #[test]
    fn distances_follow_the_java_formulas() {
        assert_eq!(coordinate_distance_for(12550824), 12550822);
        assert_eq!(switch_distance_for(12550824), 12550821);
        assert_eq!(coordinate_distance_for(1002), 998); // 1002 / 4 = 250
        assert_eq!(switch_distance_for(7), 1); // 7 / 4 = 1
    }

    #[test]
    fn adjust_pushes_only_what_is_beyond_the_distance() {
        set_distance(1000); // coordinate distance 998
        assert_eq!(adjust_coordinate(998.0), 998.0);
        assert_eq!(adjust_coordinate(-998.0), -998.0);
        assert_eq!(adjust_coordinate(999.0), 999.0 + 12550824.0 - 998.0);
        assert_eq!(adjust_coordinate(-999.0), -999.0 - 12550824.0 + 998.0);
        set_distance(BASE_DISTANCE);
        assert_eq!(adjust_coordinate(12550823.0), 12550825.0);
    }
}
