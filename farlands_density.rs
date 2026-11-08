use std::collections::BTreeMap;

/// ClassicFarlands.OVERWORLD_NOISES, as the keys Horizon's density registry uses (`namespace:path`
/// below worldgen/density_function).
const OVERWORLD_NOISES: [&str; 3] = [
    "minecraft:overworld/final_density",
    "minecraft:overworld_amplified/final_density",
    "minecraft:overworld_large_biomes/final_density",
];

/// The mod's own density function: data/classicfarlands/worldgen/density_function/farlands_final_density.json.
pub const FARLANDS_ID: &str = "classicfarlands:farlands_final_density";
const FARLANDS_FINAL_DENSITY: &str = r#"{"type":"minecraft:add","left":{"type":"minecraft:min","left":{"type":"minecraft:squeeze","input":{"type":"minecraft:interpolated","cell_size_xz":4,"cell_size_y":8,"input":{"type":"minecraft:mul","left":{"type":"minecraft:blend_density","input":{"type":"minecraft:lerp","alpha":{"type":"minecraft:gradient","axis":"y","from_coordinate":-64,"from_value":0.0,"to_coordinate":-40,"to_value":1.0},"first":0.1171875,"second":{"type":"minecraft:lerp","alpha":{"type":"minecraft:gradient","axis":"y","from_coordinate":240,"from_value":1.0,"to_coordinate":256,"to_value":0.0},"first":-0.078125,"second":{"type":"minecraft:range_choice","input":"minecraft:overworld/sloped_cheese","max_exclusive":1.5625,"min_inclusive":-1000000.0,"when_in_range":{"type":"minecraft:min","left":"minecraft:overworld/sloped_cheese","right":{"type":"minecraft:mul","left":"minecraft:overworld/caves/entrances","right":5.0}},"when_out_of_range":{"type":"minecraft:add","left":{"type":"minecraft:mul","left":{"type":"minecraft:square","input":{"type":"minecraft:noise","noise":"minecraft:cave_layer","xz_scale":1.0,"y_scale":8.0}},"right":4.0},"right":{"type":"minecraft:add","left":{"type":"minecraft:clamp","input":{"type":"minecraft:add","left":{"type":"minecraft:noise","noise":"minecraft:cave_cheese","xz_scale":1.0,"y_scale":0.6666666666666666},"right":0.27},"max":1.0,"min":-1.0},"right":{"type":"minecraft:add","left":{"type":"minecraft:mul","left":"minecraft:overworld/sloped_cheese","right":-0.64},"right":1.5}}}}}}},"right":0.64}}},"right":"minecraft:overworld/caves/noodle"},"right":{"type":"minecraft:beardifier"}}"#;

/// DensityJsonUtil.gradient
fn gradient(axis: &str, to_coordinate: i32) -> String {
    format!(
        "{{\"type\":\"minecraft:gradient\",\"axis\":\"{axis}\",\"tiling\":\"clamp_to_edge\",\"from_coordinate\":0,\"to_coordinate\":{to_coordinate},\"from_value\":0,\"to_value\":1}}"
    )
}

/// DensityJsonUtil.max
fn max(left: &str, right: &str) -> String {
    format!("{{\"type\":\"minecraft:max\",\"left\":{left},\"right\":{right}}}")
}

/// DensityJsonUtil.modifyFinalDensity: the original density inside the switch distance, the Far
/// Lands density outside it. `final_density` is the original function's JSON text, spliced in as is.
pub fn modify_final_density(final_density: &str, switch_distance: i32) -> String {
    let input = max(
        &max(
            &gradient("x", switch_distance),
            &gradient("x", switch_distance.wrapping_neg()),
        ),
        &max(
            &gradient("z", switch_distance),
            &gradient("z", switch_distance.wrapping_neg()),
        ),
    );
    format!(
        "{{\"type\":\"minecraft:range_choice\",\"input\":{input},\"min_inclusive\":-1000000.0,\"max_exclusive\":1.0,\"when_in_range\":{final_density},\"when_out_of_range\":\"{FARLANDS_ID}\"}}"
    )
}

/// Mixson's "Modify Overworld Noise" event over the loaded densities. `None` when there is nothing
/// to change: no overworld final density, or each one already goes through the Far Lands function
/// (what makes the second `from_json` call fall through, and lets a content pack that carries
/// `farlands_final_density` coexist with this mod).
pub fn apply(
    densities: &BTreeMap<String, String>,
    switch_distance: i32,
) -> Option<BTreeMap<String, String>> {
    let mut changed = densities.clone();
    let mut any = false;
    for id in OVERWORLD_NOISES {
        if let Some(original) = densities.get(id) {
            if !original.contains(FARLANDS_ID) {
                changed.insert(id.to_owned(), modify_final_density(original, switch_distance));
                any = true;
            }
        }
    }
    if !any {
        return None;
    }
    changed
        .entry(FARLANDS_ID.to_owned())
        .or_insert_with(|| FARLANDS_FINAL_DENSITY.to_owned());
    Some(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn wraps_each_overworld_final_density_and_adds_the_farlands_function() {
        let loaded = map(&[
            ("minecraft:overworld/final_density", r#"{"type":"minecraft:constant","argument":0.5}"#),
            ("minecraft:overworld_amplified/final_density", "\"minecraft:overworld/x\""),
            ("minecraft:nether/base_3d_noise", "7"),
        ]);
        let out = apply(&loaded, 12550821).expect("changed");
        let wrapped = &out["minecraft:overworld/final_density"];
        assert!(wrapped.starts_with(r#"{"type":"minecraft:range_choice","input":{"type":"minecraft:max""#));
        assert!(wrapped.contains(r#""when_in_range":{"type":"minecraft:constant","argument":0.5}"#));
        assert!(wrapped.ends_with(r#""when_out_of_range":"classicfarlands:farlands_final_density"}"#));
        assert!(wrapped.contains(r#""axis":"x","tiling":"clamp_to_edge","from_coordinate":0,"to_coordinate":12550821"#));
        assert!(wrapped.contains(r#""axis":"z","tiling":"clamp_to_edge","from_coordinate":0,"to_coordinate":-12550821"#));
        // A bare reference is spliced in as it is.
        assert!(out["minecraft:overworld_amplified/final_density"].contains(r#""when_in_range":"minecraft:overworld/x","#));
        // Not an overworld noise: untouched. Not loaded: not invented.
        assert_eq!(out["minecraft:nether/base_3d_noise"], "7");
        assert!(!out.contains_key("minecraft:overworld_large_biomes/final_density"));
        assert!(out[FARLANDS_ID].contains("minecraft:beardifier"));
    }

    #[test]
    fn already_wrapped_or_irrelevant_maps_are_left_alone() {
        let loaded = map(&[("minecraft:overworld/final_density", "{}"), ("a:b", "1")]);
        let once = apply(&loaded, 100).unwrap();
        assert_eq!(apply(&once, 100), None);
        assert_eq!(apply(&map(&[("a:b", "1")]), 100), None);
        assert_eq!(apply(&BTreeMap::new(), 100), None);
    }

    #[test]
    fn keeps_a_farlands_function_a_content_pack_already_supplies() {
        let loaded = map(&[
            ("minecraft:overworld/final_density", "{}"),
            (FARLANDS_ID, r#"{"type":"minecraft:constant","argument":1}"#),
        ]);
        let out = apply(&loaded, 100).unwrap();
        assert_eq!(out[FARLANDS_ID], loaded[FARLANDS_ID]);
        assert!(out["minecraft:overworld/final_density"].contains("range_choice"));
    }
}
