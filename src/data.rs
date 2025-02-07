use std::collections::HashMap;

use crate::il2cppdump::IL2CppDumper;

pub static mut ZOMBIE_DATA: Option<Vec<ZombieData>> = None;
pub static mut LEVEL_DATA:  Option<Vec<LevelData>>  = None;

pub enum ZombieLanes {
    Land,
    Water,
    Both,
}

pub struct ZombieData {
    pub id_name:        &'static str,
    pub allowed_lanes:  ZombieLanes,
    pub default_weight: u32,
    pub id:             Option<i32>,
    pub is_odyssey:     bool,
}

pub enum LevelType {
    Day,
    Night,
    Pool,
    Fog,
    Roof,
}

pub struct LevelData {
    pub level_type: LevelType,
    pub flags: Option<u8>,
    pub default_zombie_names: Vec<&'static str>,
    pub default_zombie_types: Vec<u32>,
}

impl Default for ZombieData {
    fn default() -> Self {
        Self {
            id_name: "",
            allowed_lanes: ZombieLanes::Land,
            default_weight: 0,
            id: None,
            is_odyssey: false,
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
        }
    }
}

pub fn init_defaults_from_dump(dump: &IL2CppDumper) {
    let mut il2cpp_syms: HashMap<String, u64> = HashMap::with_capacity(dump.methods_array.len()*3);
    
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
            ..Default::default()
        },
    ];
    
    let mut zombie_array = vec![
        ZombieData { //0
            id_name: "ZombieType::NormalZombie",
            default_weight: 4000, //weights painfully taken from InitZombieList::.cctor
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //2
            id_name: "ZombieType::ConeZombie",
            default_weight: 3000,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //3
            id_name: "ZombieType::PolevaulterZombie",
            default_weight: 3000,
            ..ZombieData::default()
        },
        ZombieData { //4
            id_name: "ZombieType::BucketZombie",
            default_weight: 2000,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //5
            id_name: "ZombieType::PaperZombie",
            default_weight: 3000,
            ..ZombieData::default()
        },
        ZombieData { //6
            id_name: "ZombieType::DancePolZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //7
            id_name: "ZombieType::DancePolZombie2",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //8
            id_name: "ZombieType::DoorZombie",
            default_weight: 2000,
            ..ZombieData::default()
        },
        ZombieData { //9
            id_name: "ZombieType::FootballZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //A
            id_name: "ZombieType::JacksonZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //B
            id_name: "ZombieType::ZombieDuck",
            default_weight: 0,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //C
            id_name: "ZombieType::ConeZombieDuck",
            default_weight: 0,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D
            id_name: "ZombieType::BucketZombieDuck",
            default_weight: 0,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //E
            id_name: "ZombieType::SubmarineZombie",
            default_weight: 750,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //F
            id_name: "ZombieType::ElitePaperZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //10
            id_name: "ZombieType::DriverZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //11
            id_name: "ZombieType::SnorkleZombie",
            default_weight: 1500,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //12
            id_name: "ZombieType::SuperDriver",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //13
            id_name: "ZombieType::Dolphinrider",
            default_weight: 1500,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //14
            id_name: "ZombieType::DrownZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //15
            id_name: "ZombieType::DollDiamond",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //16
            id_name: "ZombieType::DollGold",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //17
            id_name: "ZombieType::DollSilver",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //18
            id_name: "ZombieType::JackboxZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //19
            id_name: "ZombieType::BalloonZombie",
            default_weight: 1500,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1A
            id_name: "ZombieType::KirovZombie",
            default_weight: 750,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1B
            id_name: "ZombieType::SnowDolphinrider",
            default_weight: 1000,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //1C
            id_name: "ZombieType::MinerZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //1D
            id_name: "ZombieType::IronBallonZombie",
            default_weight: 1000,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //1E
            id_name: "ZombieType::SuperJackboxZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //1F
            id_name: "ZombieType::CatapultZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //20
            id_name: "ZombieType::PogoZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //21
            id_name: "ZombieType::LadderZombie",
            default_weight: 1500,
            ..ZombieData::default()
        },
        ZombieData { //22
            id_name: "ZombieType::SuperPogoZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //23
            id_name: "ZombieType::Gargantuar",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //24
            id_name: "ZombieType::RedGargantuar",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //25
            id_name: "ZombieType::ImpZombie",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //26
            id_name: "ZombieType::IronGargantuar",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //27
            id_name: "ZombieType::IronRedGargantuar",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //28
            id_name: "ZombieType::MachineNutZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //29
            id_name: "ZombieType::SilverZombie",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //2A
            id_name: "ZombieType::GoldZombie",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //2B
            id_name: "ZombieType::SuperGargantuar",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //2C
            id_name: "ZombieType::ZombieBoss",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //2D
            id_name: "ZombieType::BungiZombie", //can go on water and land, but we don't want this for logic purposes
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //2E
            id_name: "ZombieType::ZombieBoss2",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //64
            id_name: "ZombieType::PeaShooterZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //65
            id_name: "ZombieType::CherryShooterZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //66
            id_name: "ZombieType::SuperCherryShooterZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //67
            id_name: "ZombieType::WallNutZombie",
            default_weight: 2000,
            ..ZombieData::default()
        },
        ZombieData { //68
            id_name: "ZombieType::CherryPaperZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //69
            id_name: "ZombieType::RandomZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //6A
            id_name: "ZombieType::BucketNutZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //6B
            id_name: "ZombieType::CherryNutZombie",
            default_weight: 2000,
            ..ZombieData::default()
        },
        ZombieData { //6C
            id_name: "ZombieType::IronPeaZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //6D
            id_name: "ZombieType::TallNutFootballZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //6E
            id_name: "ZombieType::RandomPlusZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //6F
            id_name: "ZombieType::TallIceNutZombie",
            default_weight: 1000,
            ..ZombieData::default()
        },
        ZombieData { //70
            id_name: "ZombieType::CherryCatapultZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //71
            id_name: "ZombieType::DolphinPeaZombie",
            default_weight: 750,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //72
            id_name: "ZombieType::IronPeaDoorZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //73
            id_name: "ZombieType::SquashZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //74
            id_name: "ZombieType::JalaSquashZombie",
            default_weight: 500,
            ..ZombieData::default()
        },
        ZombieData { //75
            id_name: "ZombieType::JalapenoZombie",
            default_weight: 0,
            ..ZombieData::default()
        },
        ZombieData { //76
            id_name: "ZombieType::GatlingFootballZombie",
            default_weight: 750,
            ..ZombieData::default()
        },
        ZombieData { //C8
            id_name: "ZombieType::SuperSubmarine",
            default_weight: 1000,
            allowed_lanes: ZombieLanes::Water,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //C9
            id_name: "ZombieType::JacksonDriver",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CA
            id_name: "ZombieType::FootballDrown",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CB
            id_name: "ZombieType::CherryPaperZ95",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CC
            id_name: "ZombieType::BlackFootball",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CD
            id_name: "ZombieType::SuperKirov",
            default_weight: 1000,
            is_odyssey: true,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //CE
            id_name: "ZombieType::SuperBombThrower",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //CF
            id_name: "ZombieType::QuickJacksonZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D0
            id_name: "ZombieType::QingZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D1
            id_name: "ZombieType::JackboxJumpZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D2
            id_name: "ZombieType::SuperMachineNutZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D3
            id_name: "ZombieType::LandSubmarine",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D4
            id_name: "ZombieType::UltimateGargantuar",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D5
            id_name: "ZombieType::ObsidianImpZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D6
            id_name: "ZombieType::DolphinGatlingZombie",
            default_weight: 1000,
            is_odyssey: true,
            allowed_lanes: ZombieLanes::Water,
            ..ZombieData::default()
        },
        ZombieData { //D7
            id_name: "ZombieType::DiamondRandomZombie",
            default_weight: 300,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D8
            id_name: "ZombieType::DrownpultZombie",
            default_weight: 1000,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //D9
            id_name: "ZombieType::SuperDancePolZombie",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DA
            id_name: "ZombieType::UltimateFootballDrown",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DB
            id_name: "ZombieType::UltimateMachineNutZombie",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DC
            id_name: "ZombieType::UltimateFootballZombie",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DD
            id_name: "ZombieType::UltimateKirovZombie",
            default_weight: 0,
            is_odyssey: true,
            allowed_lanes: ZombieLanes::Both,
            ..ZombieData::default()
        },
        ZombieData { //DE
            id_name: "ZombieType::UltimateJacksonDriver",
            default_weight: 0,
            is_odyssey: true,
            ..ZombieData::default()
        },
        ZombieData { //DF
            id_name: "ZombieType::UltimatePaperZombie",
            default_weight: 0,
            is_odyssey: true,
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
    
    unsafe {
        ZOMBIE_DATA = Some(zombie_array);
        LEVEL_DATA  = Some(level_array);
    }
}
