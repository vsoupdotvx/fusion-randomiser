use std::{collections::HashMap, hash::BuildHasherDefault, sync::OnceLock};

use bitflags::bitflags;
use fxhash::{FxHashMap, FxHashSet};

use crate::il2cppdump::IL2CppDumper;

pub static ZOMBIE_DATA: OnceLock<Vec<ZombieData>> = OnceLock::new();
pub static LEVEL_DATA:  OnceLock<Vec<LevelData>>  = OnceLock::new();

#[derive(PartialEq, Eq)]
pub enum ZombieLanes {
    Land,
    Water,
    Both,
}

pub struct ZombieData {
    pub zombie_type:    ZombieType,
    pub name:           &'static str,
    pub allowed_lanes:  ZombieLanes,
    pub default_weight: u32,
    pub default_points: u32,
    pub id:             Option<i32>,
    pub flags:          ZombieFlags,
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ZombieFlags: u32 {
        const NONE          =   0x0;
        const IS_ELITE      =   0x1;
        const IS_VEHICLE    =   0x2;
        const DOES_NOT_EAT  =   0x4;
        const IS_METAL      =   0x8;
        const IS_ODYSSEY    =  0x10;
        const IS_BANNED     =  0x20;
        const HIGH_HEALTH   =  0x40;
        const FLIES         =  0x80;
        const V_HIGH_HEALTH = 0x100;
        const GARG_TYPE     = 0x200;
        const EVIL_DEATH    = 0x400;
        const IS_WATER      = 0x800;
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LevelType {
    Day,
    Night,
    Pool,
    Fog,
    Roof,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ZombieType {
    NormalZombie,
    ConeZombie,
    PolevaulterZombie,
    BucketZombie,
    PaperZombie,
    DancePolZombie,
    DancePolZombie2,
    DoorZombie,
    FootballZombie,
    JacksonZombie,
    ZombieDuck,
    ConeZombieDuck,
    BucketZombieDuck,
    SubmarineZombie,
    ElitePaperZombie,
    DriverZombie,
    SnorkleZombie,
    SuperDriver,
    Dolphinrider,
    DrownZombie,
    DollDiamond,
    DollGold,
    DollSilver,
    JackboxZombie,
    BalloonZombie,
    KirovZombie,
    SnowDolphinrider,
    MinerZombie,
    IronBalloonZombie,
    SuperJackboxZombie,
    CatapultZombie,
    PogoZombie,
    LadderZombie,
    SuperPogoZombie,
    Gargantuar,
    RedGargantuar,
    ImpZombie,
    IronGargantuar,
    IronRedGargantuar,
    MachineNutZombie,
    SilverZombie,
    GoldZombie,
    SuperGargantuar,
    ZombieBoss,
    BungiZombie,
    ZombieBoss2,
    SnowZombie,
    NewYearZombie,
    SnowGunZombie,
    SnowShieldZombie,
    SnowDrownZombie,
    ProtalZombie,
    LevatationZombie,
    IceZombie,
    SnowMonsterZombie,
    TrainingDummy,
    PeaShooterZombie,
    CherryShooterZombie,
    SuperCherryShooterZombie,
    WallNutZombie,
    CherryPaperZombie,
    RandomZombie,
    BucketNutZombie,
    CherryNutZombie,
    IronPeaZombie,
    TallNutFootballZombie,
    RandomPlusZombie,
    TallIceNutZombie,
    CherryCatapultZombie,
    DolphinPeaZombie,
    IronPeaDoorZombie,
    SquashZombie,
    JalaSquashZombie,
    JalapenoZombie,
    GatlingFootballZombie,
    IronBalloonZombie2,
    SuperSubmarine,
    JacksonDriver,
    FootballDrown,
    CherryPaperZ95,
    BlackFootball,
    SuperKirov,
    SuperBombThrower,
    QuickJacksonZombie,
    QingZombie,
    JackboxJumpZombie,
    SuperMachineNutZombie,
    LandSubmarine,
    UltimateGargantuar,
    ObsidianImpZombie,
    DolphinGatlingZombie,
    DiamondRandomZombie,
    DrownpultZombie,
    SuperDancePolZombie,
    UltimateFootballDrown,
    UltimateMachineNutZombie,
    UltimateFootballZombie,
    UltimateKirovZombie,
    UltimateJacksonDriver,
    UltimatePaperZombie,
    UltimateJackboxZombie,
    GatlingBlackFootball,
    LegionZombie,
    IceClawZombie,
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
    Blover,
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
    pub default_zombie_types: Vec<ZombieType>,
    pub default_zombie_ids: Vec<u32>,
    pub conveyor_plants: Option<FxHashSet<Unlockable>>,
}

impl Default for ZombieData {
    fn default() -> Self {
        Self {
            zombie_type: ZombieType::NormalZombie,
            name: "",
            allowed_lanes: ZombieLanes::Land,
            default_weight: 0,
            default_points: 1,
            id: None,
            flags: ZombieFlags::NONE,
        }
    }
}

impl Default for LevelData {
    fn default() -> Self {
        Self {
            level_type: LevelType::Day,
            flags: Some(1),
            default_zombie_types: vec![ZombieType::NormalZombie],
            default_zombie_ids: Vec::new(),
            conveyor_plants: None,
        }
    }
}

pub const COOLDOWN_TABLE: [f32; 41] = [
    7.5, 7.5, 50., 30., 30., 7.5,
    7.5, 7.5, 30., 7.5, 50., 50.,
    7.5, 30., 7.5, 30., 50., 7.5, 7.5,
    7.5, 30., 7.5, 15., 7.5, 30., 7.5,
    7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5,
    50., 50., 15., 50., 50., 50., 50., 50.,
];

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
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
            ],
            ..Default::default()
        },
        LevelData { //3
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
            ],
            ..Default::default()
        },
        LevelData { //4
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PolevaulterZombie,
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //5
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PeaShooterZombie,
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //6
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DollDiamond,
            ],
            flags: Some(2),
            ..Default::default()
        },
        LevelData { //7
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DollDiamond,
                ZombieType::CherryPaperZombie,
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //8
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::BucketZombie,
                ZombieType::DoorZombie,
                ZombieType::BucketNutZombie,
                ZombieType::IronPeaZombie,
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //9
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DollDiamond,
                ZombieType::CherryPaperZombie,
                ZombieType::BucketNutZombie,
                ZombieType::CherryNutZombie,
                ZombieType::PolevaulterZombie,
            ],
            flags: Some(3),
            ..Default::default()
        },
        LevelData { //A
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //B
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::DoorZombie,
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //C
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PaperZombie,
                ZombieType::FootballZombie,
            ],
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //D
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PaperZombie,
                ZombieType::FootballZombie,
                ZombieType::PolevaulterZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //E
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PaperZombie,
                ZombieType::FootballZombie,
                ZombieType::PolevaulterZombie,
                ZombieType::BucketZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //F
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PaperZombie,
                ZombieType::FootballZombie,
                ZombieType::DancePolZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //10
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PaperZombie,
                ZombieType::FootballZombie,
                ZombieType::DancePolZombie,
                ZombieType::JacksonZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //11
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PaperZombie,
                ZombieType::TallNutFootballZombie,
                ZombieType::DancePolZombie,
                ZombieType::JacksonZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //12
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PaperZombie,
                ZombieType::TallNutFootballZombie,
                ZombieType::DollDiamond,
                ZombieType::TallIceNutZombie,
                ZombieType::DoorZombie,
                ZombieType::FootballZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Night,
            ..Default::default()
        },
        LevelData { //13
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
            ],
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //14
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::FootballZombie,
            ],
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //15
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::SnorkleZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //16
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PolevaulterZombie,
                ZombieType::PaperZombie,
                ZombieType::SubmarineZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //17
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PaperZombie,
                ZombieType::SubmarineZombie,
                ZombieType::DriverZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //18
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DoorZombie,
                ZombieType::SubmarineZombie,
                ZombieType::SuperDriver,
            ],
            flags: Some(3),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //19
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DoorZombie,
                ZombieType::SubmarineZombie,
                ZombieType::SuperDriver,
                ZombieType::DrownZombie,
            ],
            flags: Some(4),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //1A
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::SubmarineZombie,
                ZombieType::SuperDriver,
                ZombieType::DrownZombie,
                ZombieType::DriverZombie,
            ],
            flags: Some(4),
            level_type: LevelType::Pool,
            ..Default::default()
        },
        LevelData { //1B
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::Dolphinrider,
                ZombieType::DriverZombie,
                ZombieType::SubmarineZombie,
                ZombieType::ElitePaperZombie,
                ZombieType::DrownZombie,
                ZombieType::SuperDriver,
                ZombieType::SnorkleZombie,
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
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
            ],
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1D
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::JackboxZombie,
            ],
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1E
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PaperZombie,
                ZombieType::BalloonZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //1F
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PaperZombie,
                ZombieType::BalloonZombie,
                ZombieType::DoorZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //20
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::JackboxZombie,
                ZombieType::BucketZombie,
                ZombieType::KirovZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //21
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::JackboxZombie,
                ZombieType::BalloonZombie,
                ZombieType::PolevaulterZombie,
                ZombieType::MinerZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //22
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::JackboxZombie,
                ZombieType::BalloonZombie,
                ZombieType::MinerZombie,
                ZombieType::SnowDolphinrider,
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //23
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::DoorZombie,
                ZombieType::BucketNutZombie,
                ZombieType::KirovZombie,
                ZombieType::SuperJackboxZombie,
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            ..Default::default()
        },
        LevelData { //24
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::JackboxZombie,
                ZombieType::BalloonZombie,
                ZombieType::MinerZombie,
                ZombieType::SnowDolphinrider,
                ZombieType::KirovZombie,
                ZombieType::SuperJackboxZombie,
                ZombieType::IronBalloonZombie,
            ],
            flags: Some(4),
            level_type: LevelType::Fog,
            conveyor_plants: Some([
                Unlockable::LilyPad,
                Unlockable::SeaShroom,
                Unlockable::StarFruit,
                Unlockable::Cactus,
                Unlockable::Plantern,
                Unlockable::Blover,
                Unlockable::Pumpkin,
                Unlockable::IceShroom,
                Unlockable::Magnetshroom,
            ].into_iter().collect()),
            ..Default::default()
        },
        LevelData { //25
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
            ],
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //26
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::PogoZombie,
            ],
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //27
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //28
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::BucketNutZombie,
                ZombieType::DoorZombie,
                ZombieType::LadderZombie,
            ],
            flags: Some(2),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //29
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::LadderZombie,
                ZombieType::CatapultZombie,
                ZombieType::MachineNutZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2A
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::CatapultZombie,
                ZombieType::SuperPogoZombie,
            ],
            flags: Some(3),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2B
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::BucketNutZombie,
                ZombieType::Gargantuar,
            ],
            flags: Some(4),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2C
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::BucketZombie,
                ZombieType::PogoZombie,
                ZombieType::IronGargantuar,
                ZombieType::CherryCatapultZombie,
            ],
            flags: Some(4),
            level_type: LevelType::Roof,
            ..Default::default()
        },
        LevelData { //2D
            default_zombie_types: vec![
                ZombieType::NormalZombie,
                ZombieType::ConeZombie,
                ZombieType::CherryCatapultZombie,
                ZombieType::PogoZombie,
                ZombieType::LadderZombie,
                ZombieType::CatapultZombie,
                ZombieType::Gargantuar,
                ZombieType::IronGargantuar,
                ZombieType::SuperPogoZombie,
                ZombieType::MachineNutZombie,
                ZombieType::BungiZombie,
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
            zombie_type: ZombieType::NormalZombie,
            name: "Normal",
            default_weight: 4000, //weights painfully taken from InitZombieList::.cctor
            default_points: 1, //wavepoints slightly less painfully taken from jump tables in InitZombieList::AddZombieToList
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //2
            zombie_type: ZombieType::ConeZombie,
            name: "Cone",
            default_weight: 3000,
            default_points: 2,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //3
            zombie_type: ZombieType::PolevaulterZombie,
            name: "Vaulter",
            default_weight: 3000,
            default_points: 2,
            ..ZombieData::default()
        },
        ZombieData { //4
            zombie_type: ZombieType::BucketZombie,
            name: "Bucket",
            default_weight: 2000,
            default_points: 4,
            flags: ZombieFlags::IS_METAL,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //5
            zombie_type: ZombieType::PaperZombie,
            name: "Newspaper",
            default_weight: 3000,
            default_points: 2,
            ..ZombieData::default()
        },
        ZombieData { //6
            zombie_type: ZombieType::DancePolZombie,
            name: "Michael",
            default_weight: 750,
            default_points: 6,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::EVIL_DEATH | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //7
            zombie_type: ZombieType::DancePolZombie2,
            name: "Backup dancer",
            default_weight: 0,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //8
            zombie_type: ZombieType::DoorZombie,
            name: "Screen door",
            default_weight: 2000,
            default_points: 4,
            flags: ZombieFlags::IS_METAL,
            ..ZombieData::default()
        },
        ZombieData { //9
            zombie_type: ZombieType::FootballZombie,
            name: "Football",
            default_weight: 1500,
            default_points: 4,
            flags: ZombieFlags::IS_METAL | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //A
            zombie_type: ZombieType::JacksonZombie,
            name: "Dark Michael",
            default_weight: 500,
            default_points: 10,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::EVIL_DEATH | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //B
            zombie_type: ZombieType::ZombieDuck,
            name: "Ducky tube",
            default_weight: 0,
            default_points: 1,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //C
            zombie_type: ZombieType::ConeZombieDuck,
            name: "Ducky tube cone",
            default_weight: 0,
            default_points: 1,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D
            zombie_type: ZombieType::BucketZombieDuck,
            name: "Ducky tube bucket",
            default_weight: 0,
            default_points: 1,
            flags: ZombieFlags::IS_METAL,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //E
            zombie_type: ZombieType::SubmarineZombie,
            name: "Submarine",
            default_weight: 750,
            default_points: 7,
            flags: ZombieFlags::DOES_NOT_EAT | ZombieFlags::HIGH_HEALTH,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //F
            zombie_type: ZombieType::ElitePaperZombie,
            name: "Book zombie",
            default_weight: 750,
            default_points: 6,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //10
            zombie_type: ZombieType::DriverZombie,
            name: "Zomboni",
            default_weight: 1000,
            default_points: 7,
            flags: ZombieFlags::IS_VEHICLE | ZombieFlags::DOES_NOT_EAT | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //11
            zombie_type: ZombieType::SnorkleZombie,
            name: "Snorkle",
            default_weight: 1500,
            default_points: 3,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //12
            zombie_type: ZombieType::SuperDriver,
            name: "Bobsled zomboni",
            default_weight: 750,
            default_points: 7,
            flags: ZombieFlags::IS_VEHICLE | ZombieFlags::DOES_NOT_EAT | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //13
            zombie_type: ZombieType::Dolphinrider,
            name: "Dolphin",
            default_weight: 1500,
            default_points: 3,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //14
            zombie_type: ZombieType::DrownZombie,
            name: "Trident",
            default_weight: 1500,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //15
            zombie_type: ZombieType::DollDiamond,
            name: "Diamond dude",
            default_weight: 750,
            default_points: 6,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::V_HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //16
            zombie_type: ZombieType::DollGold,
            name: "Gold guy",
            default_weight: 750,
            default_points: 5,
            flags: ZombieFlags::V_HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //17
            zombie_type: ZombieType::DollSilver,
            name: "Silver individual",
            default_weight: 750,
            default_points: 4,
            flags: ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //18
            zombie_type: ZombieType::JackboxZombie,
            name: "Jack",
            default_weight: 1500,
            default_points: 3,
            ..ZombieData::default()
        },
        ZombieData { //19
            zombie_type: ZombieType::BalloonZombie,
            name: "Balloon",
            default_weight: 1500,
            default_points: 2,
            allowed_lanes: ZombieLanes::Both,
            flags: ZombieFlags::FLIES | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //1A
            zombie_type: ZombieType::KirovZombie,
            name: "Kirov",
            default_weight: 750,
            default_points: 7,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::FLIES | ZombieFlags::DOES_NOT_EAT,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1B
            zombie_type: ZombieType::SnowDolphinrider,
            name: "Yeti dolphin",
            default_weight: 1000,
            default_points: 4,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //1C
            zombie_type: ZombieType::MinerZombie,
            name: "Miner",
            default_weight: 1500,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //1D
            zombie_type: ZombieType::IronBalloonZombie,
            name: "Metal balloon",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_METAL | ZombieFlags::HIGH_HEALTH | ZombieFlags::FLIES | ZombieFlags::DOES_NOT_EAT,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1E
            zombie_type: ZombieType::SuperJackboxZombie,
            name: "Super jack",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_METAL | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //1F
            zombie_type: ZombieType::CatapultZombie,
            name: "Catapult",
            default_weight: 1000,
            default_points: 7,
            flags: ZombieFlags::IS_VEHICLE | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //20
            zombie_type: ZombieType::PogoZombie,
            name: "Pogo",
            default_weight: 1500,
            default_points: 4,
            flags: ZombieFlags::IS_METAL | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //21
            zombie_type: ZombieType::LadderZombie,
            name: "Ladder",
            default_weight: 1500,
            default_points: 5,
            flags: ZombieFlags::IS_METAL,
            ..ZombieData::default()
        },
        ZombieData { //22
            zombie_type: ZombieType::SuperPogoZombie,
            name: "Melon pogo",
            default_weight: 1000,
            default_points: 6,
            flags: ZombieFlags::DOES_NOT_EAT | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //23
            zombie_type: ZombieType::Gargantuar,
            name: "Garg",
            default_weight: 750,
            default_points: 8,
            flags: ZombieFlags::GARG_TYPE,
            ..ZombieData::default()
        },
        ZombieData { //24
            zombie_type: ZombieType::RedGargantuar,
            name: "Giga garg",
            default_weight: 500,
            default_points: 8,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::IS_ELITE | ZombieFlags::GARG_TYPE,
            ..ZombieData::default()
        },
        ZombieData { //25
            zombie_type: ZombieType::ImpZombie,
            name: "Imp",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //26
            zombie_type: ZombieType::IronGargantuar,
            name: "Iron garg",
            default_weight: 750,
            default_points: 8,
            flags: ZombieFlags::IS_METAL | ZombieFlags::IS_ELITE | ZombieFlags::GARG_TYPE,
            ..ZombieData::default()
        },
        ZombieData { //27
            zombie_type: ZombieType::IronRedGargantuar,
            name: "Giga iron garg",
            default_weight: 500,
            default_points: 8,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::IS_ELITE | ZombieFlags::GARG_TYPE | ZombieFlags::IS_METAL,
            ..ZombieData::default()
        },
        ZombieData { //28
            zombie_type: ZombieType::MachineNutZombie,
            name: "Zomnut",
            default_weight: 750,
            default_points: 8,
            flags: ZombieFlags::DOES_NOT_EAT | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //29
            zombie_type: ZombieType::SilverZombie,
            name: "Silver zombie",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2A
            zombie_type: ZombieType::GoldZombie,
            name: "Gold zombie",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2B
            zombie_type: ZombieType::SuperGargantuar,
            name: "Gladiantaur",
            default_weight: 500,
            default_points: 10,
            flags: ZombieFlags::IS_BANNED | ZombieFlags::IS_ODYSSEY | ZombieFlags::GARG_TYPE,
            ..ZombieData::default()
        },
        ZombieData { //2C
            zombie_type: ZombieType::ZombieBoss,
            name: "Zomboss",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2D
            zombie_type: ZombieType::BungiZombie, //can go on water and land, but we don't want this for logic purposes
            name: "Bungie",
            default_weight: 1000,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //2E
            zombie_type: ZombieType::ZombieBoss2,
            name: "Zomboss 2",
            default_weight: 0,
            default_points: 0,
            ..ZombieData::default()
        },
        ZombieData { //2F
            zombie_type: ZombieType::SnowZombie,
            name: "Yeti",
            default_weight: 750,
            default_points: 5,
            flags: ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //30
            zombie_type: ZombieType::NewYearZombie,
            name: "New year zombie",
            default_weight: 750,
            default_points: 8,
            flags: ZombieFlags::V_HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //31
            zombie_type: ZombieType::SnowGunZombie,
            name: "Snowblower",
            default_weight: 1000,
            default_points: 3,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //32
            zombie_type: ZombieType::SnowShieldZombie,
            name: "Shield guy",
            default_weight: 1500,
            default_points: 3,
            flags: ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //33
            zombie_type: ZombieType::SnowDrownZombie,
            name: "Snow trident",
            default_weight: 1500,
            default_points: 3,
            flags: ZombieFlags::NONE,
            ..ZombieData::default()
        },
        ZombieData { //34
            zombie_type: ZombieType::ProtalZombie,
            name: "Portal guy",
            default_weight: 500,
            default_points: 3,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //35
            zombie_type: ZombieType::LevatationZombie,
            name: "Snow UFO",
            default_weight: 750,
            default_points: 7,
            flags: ZombieFlags::FLIES,
            ..ZombieData::default()
        },
        ZombieData { //36
            zombie_type: ZombieType::TrainingDummy,
            name: "Dummy",
            default_weight: 0,
            default_points: 1,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //37
            zombie_type: ZombieType::IceZombie,
            name: "Ice cube",
            default_weight: 3000,
            default_points: 7,
            flags: ZombieFlags::NONE,
            ..ZombieData::default()
        },
        ZombieData { //38
            zombie_type: ZombieType::SnowMonsterZombie,
            name: "Fluffy creature",
            default_weight: 750, //actually has a weight of 0, but it looks cute so i'm allowing it to spawn
            default_points: 1,
            flags: ZombieFlags::NONE,
            ..ZombieData::default()
        },
        ZombieData { //64
            zombie_type: ZombieType::PeaShooterZombie,
            name: "Peashooter zombie",
            default_weight: 1000,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //65
            zombie_type: ZombieType::CherryShooterZombie,
            name: "Cherry shooter zombie",
            default_weight: 750,
            default_points: 3,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //66
            zombie_type: ZombieType::SuperCherryShooterZombie,
            name: "Super cherry shooter zombie",
            default_weight: 750,
            default_points: 4,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //67
            zombie_type: ZombieType::WallNutZombie,
            name: "Wallnut zombie",
            default_weight: 2000,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //68
            zombie_type: ZombieType::CherryPaperZombie,
            name: "Cherry paper zombie",
            default_weight: 500,
            default_points: 8,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //69
            zombie_type: ZombieType::RandomZombie,
            name: "Random zombie",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //6A
            zombie_type: ZombieType::BucketNutZombie,
            name: "Bucket nut zombie",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::HIGH_HEALTH | ZombieFlags::IS_METAL,
            ..ZombieData::default()
        },
        ZombieData { //6B
            zombie_type: ZombieType::CherryNutZombie,
            name: "Cherry nut zombie",
            default_weight: 2000,
            default_points: 5,
            ..ZombieData::default()
        },
        ZombieData { //6C
            zombie_type: ZombieType::IronPeaZombie,
            name: "Iron pea zombie",
            default_weight: 500,
            default_points: 5,
            flags: ZombieFlags::IS_ELITE | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //6D
            zombie_type: ZombieType::TallNutFootballZombie,
            name: "Tallnut football",
            default_weight: 1000,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::IS_METAL | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //6E
            zombie_type: ZombieType::RandomPlusZombie,
            name: "Gold random",
            default_weight: 500,
            default_points: 7,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //6F
            zombie_type: ZombieType::TallIceNutZombie,
            name: "Ice tallnut zombie",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //70
            zombie_type: ZombieType::CherryCatapultZombie,
            name: "Cherry catapult",
            default_weight: 750,
            default_points: 10,
            flags: ZombieFlags::EVIL_DEATH | ZombieFlags::IS_VEHICLE | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //71
            zombie_type: ZombieType::DolphinPeaZombie,
            name: "Peashooter dolphin",
            default_weight: 750,
            default_points: 4,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //72
            zombie_type: ZombieType::IronPeaDoorZombie,
            name: "Iron pea door zombie",
            default_weight: 500,
            default_points: 5,
            flags: ZombieFlags::IS_METAL | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //73
            zombie_type: ZombieType::SquashZombie,
            name: "Squash zombie",
            default_weight: 750,
            default_points: 4,
            ..ZombieData::default()
        },
        ZombieData { //74
            zombie_type: ZombieType::JalaSquashZombie,
            name: "Jalapeno squash zombie",
            default_weight: 500,
            default_points: 8,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //75
            zombie_type: ZombieType::JalapenoZombie,
            name: "Jalapeno zombie",
            default_weight: 0,
            default_points: 1,
            ..ZombieData::default()
        },
        ZombieData { //76
            zombie_type: ZombieType::GatlingFootballZombie,
            name: "Gatling football",
            default_weight: 750,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::IS_METAL | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //77
            zombie_type: ZombieType::IronBalloonZombie2,
            name: "Iron pea balloon",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::FLIES | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //C8
            zombie_type: ZombieType::SuperSubmarine,
            name: "Super submarine",
            default_weight: 1000,
            default_points: 5,
            allowed_lanes: ZombieLanes::Water,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::DOES_NOT_EAT | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //C9
            zombie_type: ZombieType::JacksonDriver,
            name: "Dark Michael zomboni",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH | ZombieFlags::IS_VEHICLE | ZombieFlags::DOES_NOT_EAT,
            ..ZombieData::default()
        },
        ZombieData { //CA
            zombie_type: ZombieType::FootballDrown,
            name: "Trident football",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //CB
            zombie_type: ZombieType::CherryPaperZ95,
            name: "Super cherry newspaper",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //CC
            zombie_type: ZombieType::BlackFootball,
            name: "Rugby zombie",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::V_HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //CD
            zombie_type: ZombieType::SuperKirov,
            name: "Super kirov",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::DOES_NOT_EAT | ZombieFlags::FLIES,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //CE
            zombie_type: ZombieType::SuperBombThrower,
            name: "Honestly not sure what this individual is",
            default_weight: 0,
            default_points: 1,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //CF
            zombie_type: ZombieType::QuickJacksonZombie,
            name: "Bright Michael",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //D0
            zombie_type: ZombieType::QingZombie,
            name: "Qing zombie",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //D1
            zombie_type: ZombieType::JackboxJumpZombie,
            name: "Jack melon pogo",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::DOES_NOT_EAT | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //D2
            zombie_type: ZombieType::SuperMachineNutZombie,
            name: "Super zomnut",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::DOES_NOT_EAT | ZombieFlags::EVIL_DEATH,
            ..ZombieData::default()
        },
        ZombieData { //D3
            zombie_type: ZombieType::LandSubmarine,
            name: "Land submarine",
            default_weight: 0,
            default_points: 1,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //D4
            zombie_type: ZombieType::UltimateGargantuar,
            name: "Ultra garg",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //D5
            zombie_type: ZombieType::ObsidianImpZombie,
            name: "Obsidian imp",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //D6
            zombie_type: ZombieType::DolphinGatlingZombie,
            name: "Gatling dolphin",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::EVIL_DEATH,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D7
            zombie_type: ZombieType::DiamondRandomZombie,
            name: "Diamond random",
            default_weight: 300,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::V_HIGH_HEALTH,
            ..ZombieData::default()
        },
        ZombieData { //D8
            zombie_type: ZombieType::DrownpultZombie,
            name: "Trident catapult",
            default_weight: 1000,
            default_points: 5,
            flags: ZombieFlags::IS_ODYSSEY | ZombieFlags::IS_VEHICLE,
            ..ZombieData::default()
        },
        ZombieData { //D9
            zombie_type: ZombieType::SuperDancePolZombie,
            name: "Honestly not sure what this guy is either",
            default_weight: 0,
            default_points: 1,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //DA
            zombie_type: ZombieType::UltimateFootballDrown,
            name: "Ultra trident football",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //DB
            zombie_type: ZombieType::UltimateMachineNutZombie,
            name: "Giga super zomnut",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //DC
            zombie_type: ZombieType::UltimateFootballZombie,
            name: "Ultra rugby zombie",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //DD
            zombie_type: ZombieType::UltimateKirovZombie,
            name: "Ultra kirov",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //DE
            zombie_type: ZombieType::UltimateJacksonDriver,
            name: "Jackson worldwide",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //DF
            zombie_type: ZombieType::UltimatePaperZombie,
            name: "Wheelchair guy",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //E0
            zombie_type: ZombieType::UltimateJackboxZombie,
            name: "Ultra jack",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //38
            zombie_type: ZombieType::GatlingBlackFootball,
            name: "Gatling rugby zombie",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //38
            zombie_type: ZombieType::LegionZombie,
            name: "???",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
        ZombieData { //38
            zombie_type: ZombieType::IceClawZombie,
            name: "???",
            default_weight: 0,
            default_points: 10,
            flags: ZombieFlags::IS_ODYSSEY,
            ..ZombieData::default()
        },
    ];
    
    for zombie in zombie_array.iter_mut() {
        zombie.id = match il2cpp_syms.get(&format!("ZombieType::{:?}", zombie.zombie_type)) {
            Some(x) => {
                Some(*x as i32)
            },
            None => panic!("Failed to find enum ID of {:?}", zombie.zombie_type),
        };
        if zombie.allowed_lanes == ZombieLanes::Water {
            zombie.flags |= ZombieFlags::IS_WATER;
        }
    }
    
    zombie_array.sort_unstable_by_key(|zombie| unsafe {zombie.id.unwrap_unchecked()});
    
    let mut zombie_id_table: HashMap<ZombieType, u32> = HashMap::with_capacity(zombie_array.len());
    
    for (i, zombie) in zombie_array.iter_mut().enumerate() {
        zombie_id_table.insert(zombie.zombie_type, i as u32);
    }
    
    for level in level_array.iter_mut() {
        for zombie_name in &level.default_zombie_types {
            level.default_zombie_ids.push(*zombie_id_table.get(zombie_name).unwrap());
        }
    }
    
    ZOMBIE_DATA.get_or_init(|| zombie_array);
    LEVEL_DATA.get_or_init(|| level_array);
}
