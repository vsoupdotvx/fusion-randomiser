use std::{collections::HashMap, hash::BuildHasherDefault, sync::OnceLock};

use fxhash::{FxHashMap, FxHashSet};

use crate::il2cppdump::IL2CppDumper;

pub static ZOMBIE_DATA: OnceLock<Vec<ZombieData>> = OnceLock::new();
pub static LEVEL_DATA:  OnceLock<Vec<LevelData>>  = OnceLock::new();

pub enum ZombieLanes {
    Land,
    Water,
    Both,
}

pub struct ZombieData {
    pub id_name:        &'static str,
    pub name:           &'static str,
    pub allowed_lanes:  ZombieLanes,
    pub default_weight: u32,
    pub default_points: u32,
    pub id:             Option<i32>,
    pub is_elite:       bool,
    pub is_vehicle:     bool,
    pub can_hypno:      bool, //from direct hypno, not hypno scaredy/fume/wallnut
    pub is_metal:       bool,
    pub is_odyssey:     bool,
    pub is_banned:      bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LevelType {
    Day,
    Night,
    Pool,
    Fog,
    Roof,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Unlockable {
    Peashooter = 0,
    SunFlower,
    CherryBomb,
    WallNut,
    PotatoMine,
    Chomper,
    SmallPuff,
    FumeShroom,
    HypnoShroom,
    ScaredyShroom,
    IceShroom,
    DoomShroom,
    LilyPad,
    Squash,
    ThreePeater,
    Tanglekelp,
    Jalapeno,
    Caltrop,
    TorchWood,
    SeaShroom,
    Plantern,
    Cactus,
    Blower,
    StarFruit,
    Pumpkin,
    Magnetshroom,
    Cabbagepult,
    Pot,
    Cornpult,
    Garlic,
    Umbrellaleaf,
    Marigold,
    Melonpult,
    PresentZombie,
    EndoFlame,
    Present,
    TallNut,
    SpikeRock,
    CattailPlant,
    GloomShroom,
    CobCannon,
}

pub struct LevelData {
    pub level_type: LevelType,
    pub flags: Option<u8>,
    pub default_zombie_names: Vec<&'static str>,
    pub default_zombie_types: Vec<u32>,
    pub conveyor_plants: Option<FxHashSet<Unlockable>>,
}

impl Default for ZombieData {
    fn default() -> Self {
        Self {
            id_name: "",
            name: "",
            allowed_lanes: ZombieLanes::Land,
            default_weight: 0,
            default_points: 1,
            id: None,
            is_vehicle: false,
            is_elite: false,
            is_metal: false,
            can_hypno: true,
            is_odyssey: false,
            is_banned:  false,
        }
    }
}

impl Default for LevelData {
    fn default() -> Self {
        Self {
            level_type: LevelType::Day,
            flags: Some(1),
            default_zombie_names: vec!["ZombieType::NormalZombie"],
            default_zombie_types: Vec::new(),
            conveyor_plants: None,
        }
    }
}

pub fn init_defaults_from_dump(dump: &IL2CppDumper) {
    let mut il2cpp_syms: FxHashMap<String, u64> = HashMap::with_capacity_and_hasher(dump.methods_array.len()*3, BuildHasherDefault::default());
    
    dump.get_field_offsets(&mut il2cpp_syms);
    dump.get_enum_variants(&mut il2cpp_syms);
    
    il2cpp_syms.shrink_to_fit();
    
    let mut level_array = vec![
        LevelData { //1
            ..Default::default()
        },
        LevelData { //2
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
            ],
            ..Default::default()
        },
        LevelData { //3
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
            ],
            ..Default::default()
        },
        LevelData { //4
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PolevaulterZombie",
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //5
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PeaShooterZombie",
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //6
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DollDiamond"
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //7
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DollDiamond",
                "ZombieType::CherryPaperZombie",
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //8
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DoorZombie",
                "ZombieType::BucketNutZombie",
                "ZombieType::IronPeaZombie",
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //9
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DollDiamond",
                "ZombieType::CherryPaperZombie",
                "ZombieType::BucketNutZombie",
                "ZombieType::CherryNutZombie",
                "ZombieType::PolevaulterZombie",
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //A
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //B
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::DoorZombie",
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //C
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PaperZombie",
                "ZombieType::FootballZombie",
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //D
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PaperZombie",
                "ZombieType::FootballZombie",
                "ZombieType::PolevaulterZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //E
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PaperZombie",
                "ZombieType::FootballZombie",
                "ZombieType::PolevaulterZombie",
                "ZombieType::BucketZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //F
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PaperZombie",
                "ZombieType::FootballZombie",
                "ZombieType::DancePolZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //10
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PaperZombie",
                "ZombieType::FootballZombie",
                "ZombieType::DancePolZombie",
                "ZombieType::JacksonZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //11
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PaperZombie",
                "ZombieType::TallNutFootballZombie",
                "ZombieType::DancePolZombie",
                "ZombieType::JacksonZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //12
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PaperZombie",
                "ZombieType::TallNutFootballZombie",
                "ZombieType::DollDiamond",
                "ZombieType::TallIceNutZombie",
                "ZombieType::DoorZombie",
                "ZombieType::FootballZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //13
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
            ],
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //14
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::FootballZombie",
            ],
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //15
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::SnorkleZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //16
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PolevaulterZombie",
                "ZombieType::PaperZombie",
                "ZombieType::SubmarineZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //17
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PaperZombie",
                "ZombieType::SubmarineZombie",
                "ZombieType::DriverZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //18
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DoorZombie",
                "ZombieType::SubmarineZombie",
                "ZombieType::SuperDriver",
            ],
            flags: Some(3),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //19
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DoorZombie",
                "ZombieType::SubmarineZombie",
                "ZombieType::SuperDriver",
                "ZombieType::DrownZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //1A
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::SubmarineZombie",
                "ZombieType::SuperDriver",
                "ZombieType::DrownZombie",
                "ZombieType::DriverZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //1B
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::Dolphinrider",
                "ZombieType::DriverZombie",
                "ZombieType::SubmarineZombie",
                "ZombieType::ElitePaperZombie",
                "ZombieType::DrownZombie",
                "ZombieType::SuperDriver",
                "ZombieType::SnorkleZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Pool,
            conveyor_plants: Some([
                Unlockable::LilyPad,
                Unlockable::Squash,
                Unlockable::ThreePeater,
                Unlockable::Tanglekelp,
                Unlockable::Jalapeno,
                Unlockable::Caltrop,
                Unlockable::TorchWood,
                Unlockable::WallNut,
            ].into_iter().collect()),
            ..Default::default()
        },
        LevelData { //1C
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
            ],
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1D
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::JackboxZombie",
            ],
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1E
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PaperZombie",
                "ZombieType::BalloonZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1F
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PaperZombie",
                "ZombieType::BalloonZombie",
                "ZombieType::DoorZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //20
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::JackboxZombie",
                "ZombieType::BucketZombie",
                "ZombieType::KirovZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //21
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::JackboxZombie",
                "ZombieType::BalloonZombie",
                "ZombieType::PolevaulterZombie",
                "ZombieType::MinerZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //22
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::JackboxZombie",
                "ZombieType::BalloonZombie",
                "ZombieType::MinerZombie",
                "ZombieType::SnowDolphinrider",
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //23
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::DoorZombie",
                "ZombieType::BucketNutZombie",
                "ZombieType::KirovZombie",
                "ZombieType::SuperJackboxZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //24
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::JackboxZombie",
                "ZombieType::BalloonZombie",
                "ZombieType::MinerZombie",
                "ZombieType::SnowDolphinrider",
                "ZombieType::KirovZombie",
                "ZombieType::SuperJackboxZombie",
                "ZombieType::IronBallonZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            conveyor_plants: Some([
                Unlockable::LilyPad,
                Unlockable::SeaShroom,
                Unlockable::StarFruit,
                Unlockable::Cactus,
                Unlockable::Plantern,
                Unlockable::Blower,
                Unlockable::Pumpkin,
                Unlockable::IceShroom,
                Unlockable::Magnetshroom,
            ].into_iter().collect()),
            ..Default::default()
        },
        LevelData { //25
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
            ],
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //26
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::PogoZombie",
            ],
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //27
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //28
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::BucketNutZombie",
                "ZombieType::DoorZombie",
                "ZombieType::LadderZombie",
            ],
            flags: Some(2),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //29
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::LadderZombie",
                "ZombieType::CatapultZombie",
                "ZombieType::MachineNutZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2A
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::CatapultZombie",
                "ZombieType::SuperPogoZombie",
            ],
            flags: Some(3),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2B
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::BucketNutZombie",
                "ZombieType::Gargantuar",
            ],
            flags: Some(4),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2C
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::BucketZombie",
                "ZombieType::PogoZombie",
                "ZombieType::IronGargantuar",
                "ZombieType::CherryCatapultZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2D
            default_zombie_names: vec![
                "ZombieType::NormalZombie",
                "ZombieType::ConeZombie",
                "ZombieType::CherryCatapultZombie",
                "ZombieType::PogoZombie",
                "ZombieType::LadderZombie",
                "ZombieType::CatapultZombie",
                "ZombieType::Gargantuar",
                "ZombieType::IronGargantuar",
                "ZombieType::SuperPogoZombie",
                "ZombieType::MachineNutZombie",
                "ZombieType::BungiZombie",
            ],
            flags: Some(4),
            level_type: LevelType::Roof,
            conveyor_plants: Some([
                Unlockable::Pot,
                Unlockable::Cornpult,
                Unlockable::Melonpult,
                Unlockable::Cabbagepult,
                Unlockable::Umbrellaleaf,
                Unlockable::Jalapeno,
                Unlockable::IceShroom,
            ].into_iter().collect()),
            ..Default::default()
        },
    ];
    
    let mut zombie_array = vec![
        ZombieData { //0
            id_name: "ZombieType::NormalZombie",
            name: "Normal",
            default_weight: 4000, //weights painfully taken from InitZombieList::.cctor
            default_points: 1, //wavepoints slightly less painfully taken from jump tables in InitZombieList::AddZombieToList
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //2
            id_name: "ZombieType::ConeZombie",
            name: "Cone",
            default_weight: 3000,
            default_points: 2,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //3
            id_name: "ZombieType::PolevaulterZombie",
            name: "Vaulter",
            default_weight: 3000,
            default_points: 2,
            ..ZombieData::default()
        },
        ZombieData { //4
            id_name: "ZombieType::BucketZombie",
            name: "Bucket",
            default_weight: 2000,
            default_points: 4,
            is_metal: true,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //5
            id_name: "ZombieType::PaperZombie",
            name: "Newspaper",
            default_weight: 3000,
            default_points: 2,
            ..ZombieData::default()
        },
        ZombieData { //6
            id_name: "ZombieType::DancePolZombie",
            name: "Michael",
            default_weight: 750,
            default_points: 6,
            is_elite: true,
            ..ZombieData::default()
        },
        ZombieData { //7
            id_name: "ZombieType::DancePolZombie2",
            name: "Backup dancer",
            default_weight: 0,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //8
            id_name: "ZombieType::DoorZombie",
            name: "Screen door",
            default_weight: 2000,
            default_points: 4,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //9
            id_name: "ZombieType::FootballZombie",
            name: "Football",
            default_weight: 1500,
            default_points: 4,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //A
            id_name: "ZombieType::JacksonZombie",
            name: "Dark Michael",
            default_weight: 500,
            default_points: 10,
            is_elite: true,
            ..ZombieData::default()
        },
        ZombieData { //B
            id_name: "ZombieType::ZombieDuck",
            name: "Ducky tube",
            default_weight: 0,
            default_points: 1,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //C
            id_name: "ZombieType::ConeZombieDuck",
            name: "Ducky tube cone",
            default_weight: 0,
            default_points: 1,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D
            id_name: "ZombieType::BucketZombieDuck",
            name: "Ducky tube bucket",
            default_weight: 0,
            default_points: 1,
            is_metal: true,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //E
            id_name: "ZombieType::SubmarineZombie",
            name: "Submarine",
            default_weight: 750,
            default_points: 7,
            can_hypno: false,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //F
            id_name: "ZombieType::ElitePaperZombie",
            name: "Book zombie",
            default_weight: 750,
            default_points: 6,
            is_elite: true,
            ..ZombieData::default()
        },
        ZombieData { //10
            id_name: "ZombieType::DriverZombie",
            name: "Zomboni",
            default_weight: 1000,
            default_points: 7,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //11
            id_name: "ZombieType::SnorkleZombie",
            name: "Snorkle",
            default_weight: 1500,
            default_points: 3,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //12
            id_name: "ZombieType::SuperDriver",
            name: "Bobsled zomboni",
            default_weight: 750,
            default_points: 7,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //13
            id_name: "ZombieType::Dolphinrider",
            name: "Dolphin",
            default_weight: 1500,
            default_points: 3,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //14
            id_name: "ZombieType::DrownZombie",
            name: "Trident",
            default_weight: 1500,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //15
            id_name: "ZombieType::DollDiamond",
            name: "Diamond dude",
            default_weight: 750,
            default_points: 6,
            is_elite: true,
            ..ZombieData::default()
        },
        ZombieData { //16
            id_name: "ZombieType::DollGold",
            name: "Gold guy",
            default_weight: 750,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //17
            id_name: "ZombieType::DollSilver",
            name: "Silver individual",
            default_weight: 750,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //18
            id_name: "ZombieType::JackboxZombie",
            name: "Bwah",
            default_weight: 1500,
            default_points: 3,
            ..ZombieData::default()
        },
        ZombieData { //19
            id_name: "ZombieType::BalloonZombie",
            name: "Balloon",
            default_weight: 1500,
            default_points: 2,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1A
            id_name: "ZombieType::KirovZombie",
            name: "Kirov",
            default_weight: 750,
            default_points: 7,
            is_elite: true,
            can_hypno: false,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1B
            id_name: "ZombieType::SnowDolphinrider",
            name: "Yeti dolphin",
            default_weight: 1000,
            default_points: 4,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //1C
            id_name: "ZombieType::MinerZombie",
            name: "Miner",
            default_weight: 1500,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //1D
            id_name: "ZombieType::IronBallonZombie",
            name: "Metal balloon",
            default_weight: 1000,
            default_points: 5,
            is_metal: true,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1E
            id_name: "ZombieType::SuperJackboxZombie",
            name: "Super bwah",
            default_weight: 1000,
            default_points: 5,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //1F
            id_name: "ZombieType::CatapultZombie",
            name: "Catapult",
            default_weight: 1000,
            default_points: 7,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //20
            id_name: "ZombieType::PogoZombie",
            name: "Pogo",
            default_weight: 1500,
            default_points: 4,
            is_metal: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //21
            id_name: "ZombieType::LadderZombie",
            name: "Ladder",
            default_weight: 1500,
            default_points: 5,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //22
            id_name: "ZombieType::SuperPogoZombie",
            name: "Melon pogo",
            default_weight: 1000,
            default_points: 6,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //23
            id_name: "ZombieType::Gargantuar",
            name: "Garg",
            default_weight: 750,
            default_points: 8,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //24
            id_name: "ZombieType::RedGargantuar",
            name: "Giga garg",
            default_weight: 500,
            default_points: 8,
            is_odyssey: true,
            is_elite: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //25
            id_name: "ZombieType::ImpZombie",
            name: "Imp",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //26
            id_name: "ZombieType::IronGargantuar",
            name: "Iron garg",
            default_weight: 750,
            default_points: 8,
            is_elite: true,
            can_hypno: false,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //27
            id_name: "ZombieType::IronRedGargantuar",
            name: "Giga iron garg",
            default_weight: 500,
            default_points: 8,
            is_elite: true,
            is_odyssey: true,
            can_hypno: false,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //28
            id_name: "ZombieType::MachineNutZombie",
            name: "Zomnut",
            default_weight: 750,
            default_points: 8,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //29
            id_name: "ZombieType::SilverZombie",
            name: "Silver zombie",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2A
            id_name: "ZombieType::GoldZombie",
            name: "Gold zombie",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2B
            id_name: "ZombieType::SuperGargantuar",
            name: "Gladiantaur",
            default_weight: 500,
            default_points: 10,
            is_banned: true,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //2C
            id_name: "ZombieType::ZombieBoss",
            name: "Zomboss",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2D
            id_name: "ZombieType::BungiZombie", //can go on water and land, but we don't want this for logic purposes
            name: "Bungie",
            default_weight: 1000,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //2E
            id_name: "ZombieType::ZombieBoss2",
            name: "Zomboss 2",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2F
            id_name: "ZombieType::SnowZombie",
            name: "Yeti",
            default_weight: 750,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //30
            id_name: "ZombieType::NewYearZombie",
            name: "New year zombie",
            default_weight: 750,
            default_points: 8,
            ..ZombieData::default()
        },
        ZombieData { //31
            id_name: "ZombieType::SnowGunZombie",
            name: "Snowblower",
            default_weight: 500,
            default_points: 8,
            is_odyssey: true, //no idea what this individual does, but they look scary in the almanac
            ..ZombieData::default()
        },
        ZombieData { //64
            id_name: "ZombieType::PeaShooterZombie",
            name: "Peashooter zombie",
            default_weight: 1000,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //65
            id_name: "ZombieType::CherryShooterZombie",
            name: "Cherry shooter zombie",
            default_weight: 750,
            default_points: 3,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //66
            id_name: "ZombieType::SuperCherryShooterZombie",
            name: "Super cherry shooter zombie",
            default_weight: 750,
            default_points: 4,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //67
            id_name: "ZombieType::WallNutZombie",
            name: "Wallnut zombie",
            default_weight: 2000,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //68
            id_name: "ZombieType::CherryPaperZombie",
            name: "Cherry paper zombie",
            default_weight: 500,
            default_points: 8,
            is_elite: true,
            ..ZombieData::default()
        },
        ZombieData { //69
            id_name: "ZombieType::RandomZombie",
            name: "Random zombie",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //6A
            id_name: "ZombieType::BucketNutZombie",
            name: "Bucket nut zombie",
            default_weight: 1000,
            default_points: 5,
            is_elite: true,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //6B
            id_name: "ZombieType::CherryNutZombie",
            name: "Cherry nut zombie",
            default_weight: 2000,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //6C
            id_name: "ZombieType::IronPeaZombie",
            name: "Iron pea zombie",
            default_weight: 500,
            default_points: 5,
            is_elite: true,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //6D
            id_name: "ZombieType::TallNutFootballZombie",
            name: "Tallnut football",
            default_weight: 1000,
            default_points: 10,
            is_elite: true,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //6E
            id_name: "ZombieType::RandomPlusZombie",
            name: "Gold random",
            default_weight: 500,
            default_points: 7,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //6F
            id_name: "ZombieType::TallIceNutZombie",
            name: "Ice tallnut zombie",
            default_weight: 1000,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //70
            id_name: "ZombieType::CherryCatapultZombie",
            name: "Cherry catapult",
            default_weight: 750,
            default_points: 10,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //71
            id_name: "ZombieType::DolphinPeaZombie",
            name: "Peashooter dolphin",
            default_weight: 750,
            default_points: 4,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //72
            id_name: "ZombieType::IronPeaDoorZombie",
            name: "Iron pea door zombie",
            default_weight: 500,
            default_points: 5,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //73
            id_name: "ZombieType::SquashZombie",
            name: "Squash zombie",
            default_weight: 750,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //74
            id_name: "ZombieType::JalaSquashZombie",
            name: "Jalapeno squash zombie",
            default_weight: 500,
            default_points: 8,
            is_odyssey: true, //this guy wasn't odyssey before despite being probably the most dangerous zombie here
            ..ZombieData::default()
        },
        ZombieData { //75
            id_name: "ZombieType::JalapenoZombie",
            name: "Jalapeno zombie",
            default_weight: 0,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //76
            id_name: "ZombieType::GatlingFootballZombie",
            name: "Gatling football",
            default_weight: 750,
            default_points: 10,
            is_odyssey: true,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //77
            id_name: "ZombieType::IronBallonZombie2", //no almanac entry, hopefully not evil
            name: "Iron pea balloon",
            default_weight: 1000,
            default_points: 5,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //C8
            id_name: "ZombieType::SuperSubmarine",
            name: "Super submarine",
            default_weight: 1000,
            default_points: 5,
            allowed_lanes: ZombieLanes::Water,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //C9
            id_name: "ZombieType::JacksonDriver",
            name: "Dark Michael zomboni",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //CA
            id_name: "ZombieType::FootballDrown",
            name: "Trident football",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CB
            id_name: "ZombieType::CherryPaperZ95",
            name: "Super cherry newspaper",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //CC
            id_name: "ZombieType::BlackFootball",
            name: "Rugby zombie",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CD
            id_name: "ZombieType::SuperKirov",
            name: "Super kirov",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            can_hypno: false,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //CE
            id_name: "ZombieType::SuperBombThrower",
            name: "Honestly not sure what this individual is",
            default_weight: 0,
            default_points: 1,
            is_odyssey: true,
            is_vehicle: true, //sounds like a catapult type zombie, though I haven't confirmed this
            ..ZombieData::default()
        },
        ZombieData { //CF
            id_name: "ZombieType::QuickJacksonZombie",
            name: "Bright Michael",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D0
            id_name: "ZombieType::QingZombie",
            name: "Qing zombie",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D1
            id_name: "ZombieType::JackboxJumpZombie",
            name: "Bwah melon pogo",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //D2
            id_name: "ZombieType::SuperMachineNutZombie",
            name: "Super zomnut",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //D3
            id_name: "ZombieType::LandSubmarine",
            name: "Land submarine",
            default_weight: 0,
            default_points: 1,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //D4
            id_name: "ZombieType::UltimateGargantuar",
            name: "Ultra garg",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //D5
            id_name: "ZombieType::ObsidianImpZombie",
            name: "Obsidian imp",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D6
            id_name: "ZombieType::DolphinGatlingZombie",
            name: "Gatling dolphin",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D7
            id_name: "ZombieType::DiamondRandomZombie",
            name: "Diamond random",
            default_weight: 300,
            default_points: 5,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D8
            id_name: "ZombieType::DrownpultZombie",
            name: "Trident catapult",
            default_weight: 1000,
            default_points: 5,
            is_odyssey: true,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //D9
            id_name: "ZombieType::SuperDancePolZombie",
            name: "Honestly not sure what this guy is either",
            default_weight: 0,
            default_points: 1,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DA
            id_name: "ZombieType::UltimateFootballDrown",
            name: "Ultra trident football",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DB
            id_name: "ZombieType::UltimateMachineNutZombie",
            name: "Giga super zomnut",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            can_hypno: false,
            ..ZombieData::default()
        },
        ZombieData { //DC
            id_name: "ZombieType::UltimateFootballZombie",
            name: "Ultra rugby zombie",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            is_metal: true,
            ..ZombieData::default()
        },
        ZombieData { //DD
            id_name: "ZombieType::UltimateKirovZombie",
            name: "Ultra kirov",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            can_hypno: false,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //DE
            id_name: "ZombieType::UltimateJacksonDriver",
            name: "Jackson worldwide",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            is_vehicle: true,
            ..ZombieData::default()
        },
        ZombieData { //DF
            id_name: "ZombieType::UltimatePaperZombie",
            name: "Wheelchair guy",
            default_weight: 0,
            default_points: 10,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //E0
            id_name: "ZombieType::UltimateJackboxZombie",
            name: "Ultra bwah",
            default_weight: 0,
            default_points: 10,
            is_metal: true,
            ..ZombieData::default()
        },
    ];
    
    for zombie in zombie_array.iter_mut() {
        zombie.id = match il2cpp_syms.get(zombie.id_name) {
            Some(x) => {
                Some(*x as i32)
            },
            None => panic!("Failed to find enum ID of {}", zombie.id_name),
        }
    }
    
    zombie_array.sort_unstable_by_key(|zombie| unsafe {zombie.id.unwrap_unchecked()});
    
    let mut zombie_id_table: HashMap<&'static str, u32> = HashMap::with_capacity(zombie_array.len());
    
    for (i, zombie) in zombie_array.iter_mut().enumerate() {
        zombie_id_table.insert(zombie.id_name, i as u32);
    }
    
    for level in level_array.iter_mut() {
        for zombie_name in &level.default_zombie_names {
            level.default_zombie_types.push(*zombie_id_table.get(zombie_name).unwrap());
        }
    }
    
    ZOMBIE_DATA.get_or_init(|| zombie_array);
    LEVEL_DATA.get_or_init(|| level_array);
}
