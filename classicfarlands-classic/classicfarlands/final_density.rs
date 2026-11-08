// Mixson "Modify Overworld Noise": past the switch distance the overworld uses the Far Lands final
// density. Hooks WorldgenOverrides::apply, which Horizon runs once per worldgen bundle when a world's
// data loads. Every bundle other than the density functions goes on to the original body untouched.
//
// For the density functions this is `apply`'s own body (the data packs' entries over the bundle),
// followed by the mod's change. `port_classicfarlands_density::apply` gives back None when there is
// nothing to wrap (no overworld final density in the map, or each one already goes through the Far
// Lands function), and the map as it is then.
fn final_density(
    overrides: &WorldgenOverrides,
    bundle: &str,
    base: &BTreeMap<String, String>,
) -> std::ops::ControlFlow<BTreeMap<String, String>> {
    if bundle != "densities" {
        return std::ops::ControlFlow::Continue(());
    }
    let mut merged = base.clone();
    for (id, json) in overrides.entries(bundle) {
        merged.insert(id.to_owned(), json.to_owned());
    }
    let switch = horizon_core::noise::port_classicfarlands_state::switch_distance();
    match crate::worldgen::router::port_classicfarlands_density::apply(&merged, switch) {
        Some(changed) => std::ops::ControlFlow::Break(changed),
        None => std::ops::ControlFlow::Break(merged),
    }
}
