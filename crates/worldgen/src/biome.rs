//! Biomes: what a stretch of the surface is like. The plan lays them across
//! the world from temperature × moisture noise (DESIGN §3.2); each only sets
//! numbers (warmth, land height, hills, trees, lakes) that the relief, the
//! climate and the raster use. Snow, ice and bare rock follow from the
//! climate and the slope, not from the biome.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Ocean,
    Plains,
    Forest,
    Desert,
    Tundra,
    Jungle,
    Swamp,
}

impl Biome {
    pub const ALL: [Biome; 7] = [Biome::Ocean, Biome::Plains, Biome::Forest, Biome::Desert, Biome::Tundra, Biome::Jungle, Biome::Swamp];

    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::Plains => "plains",
            Biome::Forest => "forest",
            Biome::Desert => "desert",
            Biome::Tundra => "tundra",
            Biome::Jungle => "jungle",
            Biome::Swamp => "swamp",
        }
    }

    /// From temperature and moisture, each about −1 … 1.
    pub fn from_climate(temperature: f64, moisture: f64) -> Biome {
        if temperature < -0.35 {
            Biome::Tundra
        } else if temperature > 0.35 {
            if moisture < 0.0 { Biome::Desert } else { Biome::Jungle }
        } else if moisture < -0.3 {
            Biome::Plains
        } else if moisture > 0.45 {
            Biome::Swamp
        } else {
            Biome::Forest
        }
    }

    /// °C warmer (or colder) than the world's base climate.
    pub fn warmth(self) -> f64 {
        match self {
            Biome::Ocean => 0.0,
            Biome::Plains => 2.0,
            Biome::Forest => 0.0,
            Biome::Desert => 18.0,
            Biome::Tundra => -22.0,
            Biome::Jungle => 12.0,
            Biome::Swamp => 6.0,
        }
    }

    /// How far above sea level the land sits (cells).
    pub fn lift(self) -> f64 {
        match self {
            Biome::Ocean => 0.0,
            Biome::Plains => 40.0,
            Biome::Forest => 60.0,
            Biome::Desert => 50.0,
            Biome::Tundra => 55.0,
            Biome::Jungle => 45.0,
            Biome::Swamp => 6.0,
        }
    }

    /// How much the land rolls (cells).
    pub fn hills(self) -> f64 {
        match self {
            Biome::Ocean => 30.0,
            Biome::Plains => 45.0,
            Biome::Forest => 90.0,
            Biome::Desert => 70.0,
            Biome::Tundra => 80.0,
            Biome::Jungle => 110.0,
            Biome::Swamp => 10.0,
        }
    }

    /// Chance /256 that a tree site grows a tree.
    pub fn trees(self) -> u64 {
        match self {
            Biome::Ocean | Biome::Desert => 0,
            Biome::Plains => 45,
            Biome::Forest | Biome::Jungle => 256,
            Biome::Tundra => 70,
            Biome::Swamp => 150,
        }
    }

    /// Dense woods (+) or open meadow (−), about −1 … 1.
    pub fn lushness(self) -> f64 {
        match self {
            Biome::Forest => 0.45,
            Biome::Jungle => 0.8,
            Biome::Plains => -0.35,
            Biome::Swamp => 0.1,
            _ => 0.0,
        }
    }

    /// Chance /256 that a hollow holds a lake (the big bowls always do).
    pub fn lake_chance(self) -> u64 {
        match self {
            Biome::Ocean => 0,
            Biome::Desert => 20,
            Biome::Swamp => 230,
            Biome::Tundra => 100,
            Biome::Forest | Biome::Jungle => 80,
            Biome::Plains => 64,
        }
    }

    /// Deepest a lake gets (cells, large world); 0: dry (the odd oasis aside).
    pub fn lake_depth(self) -> f64 {
        match self {
            Biome::Ocean => 0.0,
            Biome::Desert => 0.0,
            Biome::Swamp => 22.0,
            _ => 150.0,
        }
    }
}
