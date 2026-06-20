//! A pool of 100 arbitrary world names + a deterministic "random" pick. We avoid
//! an RNG dependency: the seed (a world id) is hashed into the pool, so each
//! world gets a stable, arbitrary-looking name.

const NAMES: [&str; 100] = [
    "Aurora", "Borealis", "Cobalt", "Drift", "Ember", "Fjord", "Gossamer", "Halcyon", "Indigo",
    "Juniper", "Kestrel", "Lumen", "Marrow", "Nimbus", "Onyx", "Pinnacle", "Quartz", "Rivulet",
    "Solstice", "Tundra", "Umbra", "Vesper", "Willow", "Xenon", "Yonder", "Zephyr", "Apex",
    "Basalt", "Cinder", "Dune", "Echo", "Fathom", "Glacier", "Hollow", "Ion", "Jetty", "Kelp",
    "Lagoon", "Mesa", "Nadir", "Oasis", "Prism", "Quill", "Reef", "Spire", "Talon", "Updraft",
    "Vortex", "Wisp", "Xeric", "Yarrow", "Zenith", "Amber", "Brisk", "Crest", "Delta", "Eddy",
    "Flux", "Grove", "Haven", "Isle", "Jade", "Knoll", "Lattice", "Monsoon", "Nova", "Orbit",
    "Pylon", "Quasar", "Ridge", "Saffron", "Thistle", "Ulysses", "Verdant", "Warden", "Xylem",
    "Yield", "Zircon", "Ashen", "Beacon", "Citrine", "Dapple", "Eon", "Frost", "Glint", "Harbor",
    "Ivory", "Jasper", "Kindle", "Loft", "Mistral", "Nectar", "Opal", "Plume", "Quench", "Rune",
    "Sable", "Tarn", "Ursa", "Veil",
];

/// A stable, arbitrary name for a world id.
pub fn random_name(world: uuid::Uuid) -> String {
    NAMES[(world.as_u128() % NAMES.len() as u128) as usize].to_string()
}
