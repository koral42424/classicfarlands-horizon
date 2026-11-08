/// The Fabric mod's config file, `config/classicfarlands.toml`: same name, same `distance` key,
/// so an existing file carries over.
pub const FILE_NAME: &str = "classicfarlands.toml";

pub const DEFAULT_DISTANCE: i32 = 12550824;

/// Written when the file is missing. `disableAquifers` of the Fabric mod is not supported yet.
pub const DEFAULT_FILE: &str = "# The distance from world origin the Far Lands begin at, precise to within 4 blocks\ndistance = 12550824\n";

/// The `distance` of a config file's text: `None` when it has no usable `distance` line (the caller
/// keeps the default). Other keys, comments and tables are ignored.
pub fn parse_distance(text: &str) -> Option<i32> {
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "distance" {
            return value.trim().replace('_', "").parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_distance_the_fabric_mod_writes() {
        let file = "# The distance from world origin the Far Lands begin at, precise to within 4 blocks\ndistance = 5000\n\n# If aquifers should be disabled in the Far Lands\ndisableAquifers = true\n";
        assert_eq!(parse_distance(file), Some(5000));
    }

    #[test]
    fn tolerates_toml_spellings_and_ignores_junk() {
        assert_eq!(parse_distance("distance=12_550_824 # classic"), Some(12550824));
        assert_eq!(parse_distance("distance = -40"), Some(-40));
        assert_eq!(parse_distance("distance = lots"), None);
        assert_eq!(parse_distance("# distance = 7\nother = 1"), None);
        assert_eq!(parse_distance(""), None);
    }

    #[test]
    fn the_default_file_round_trips() {
        assert_eq!(parse_distance(DEFAULT_FILE), Some(DEFAULT_DISTANCE));
    }
}
