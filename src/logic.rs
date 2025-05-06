use std::{collections::{HashMap, HashSet}, hash::{BuildHasherDefault, Hash}, mem::transmute, ops::Not, sync::OnceLock};

use arrayvec::ArrayVec;
use fxhash::{FxHashMap, FxHashSet};
use rand::RngCore;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use smallvec::SmallVec;
use crate::{data::{LevelData, LevelType, Unlockable, ZombieFlags, ZombieLanes, ZombieType, COOLDOWN_TABLE, LEVEL_DATA}, il2cppdump::IL2CppDumper, util::hash_str};
use crate::data::ZOMBIE_DATA;

pub struct RandomisationData {
    pub level_order:   Vec<u8>,
    pub plant_order:   Vec<u8>,
    pub sound_seeds:   Option<Vec<u64>>,
    pub weights:       Option<Vec<Vec<u8>>>,
    pub firerates:     Option<Vec<Vec<u8>>>,
    pub cooldowns:     Option<Vec<Vec<u8>>>,
    pub costs:         Option<Vec<Vec<u8>>>,
    pub spawns:        Option<Vec<Vec<u8>>>,
    pub freqs:         Option<Vec<Vec<u8>>>,
    restrictions_data: Option<RestrictionsData>,
}

#[derive(Clone)]
struct LevelPlants {
    menu: Vec<(u8, u8)>,
    all: Vec<u8>,
}

struct RestrictionsData {
    frequency_cache: FxHashMap<FrequencyCacheKey, FrequencyData>,
    level_spawns: FxHashMap<u8, Vec<(u32,u32)>>,
    modified_level_spawns: FxHashMap<u8, Vec<(u32,u32)>>,
    level_plants: FxHashMap<u8, LevelPlants>,
    modified_level_plants: FxHashMap<u8, LevelPlants>,
    plant_map: FxHashMap<String, u32>,
    unlocked_plants: FxHashSet<Unlockable>,
}

#[allow(dead_code)]
enum ImpossibleReason {
    NoWaterSolution,
    InsufficientWaterSolution,
    NoPot,
    FourFlag,
    HardZombies(f64, FxHashMap<u32,u32>),
    BadPlants(f64, Vec<(Unlockable,u8,u8,u8)>),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Problem {
    Water1,
    Water2,
    Water34,
    Roof,
    Snorkle,
    DarkMicheal,
    HighHealth,
    ReallyHighHealth,
    Gargantaur,
    Balloon,
    BalloonWater,
    Kirov,
    NoPuff,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct FrequencyData {
    pub raw_averages: Vec<f32>,
    max_frequency: FxHashMap<u32, (f32, u32)>,
    first_flag_totals: FxHashMap<u32, f32>,
    first_2_flag_max: FxHashMap<u32, f32>,
    first_wave_occurence_avgs: FxHashMap<u32, u32>,
    totals: Vec<u8>,
}

#[derive(Hash, Clone, Eq, PartialEq)]
struct FrequencyCacheKey {
    spawns: Box<[(u32,u32)]>,
    level: usize,
}

struct ProblemData {
    zombie_idx: u32,
    orig_freq:  f32,
    freq:       f32,
    solutions:  FxHashSet<SolutionEntry>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct SolutionEntry {
    uses:   Option<u8>,
    negate: bool,
    night:  bool,
    weight: f32,
    plants: ArrayVec<Unlockable, 4>,
}
impl Default for SolutionEntry {
    fn default() -> Self {
        Self {
            uses: Some(1),
            negate: false,
            night:  false,
            weight: 1.,
            plants: ArrayVec::new(),
        }
    }
}
impl Not for SolutionEntry {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        self.negate = !self.negate;
        self
    }
}
impl PartialEq for SolutionEntry {
    fn eq(&self, other: &Self) -> bool {
        self.plants == other.plants
    }
}
impl Eq for SolutionEntry {}
impl Hash for SolutionEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for plant in &self.plants {
            state.write_u8(*plant as u8);
        }
        state.write_u32(self.weight.to_bits());
    }
}
impl SolutionEntry {
    #[allow(dead_code)]
    fn compute_new_plant_utilization(&self, modifiers: &LevelPlants, usage_set: &mut [f32], zombie_cooldown: f32) -> f32 { //outputs zombie _frequency_
        let solution_cooldown = self.compute_solution_cooldown(modifiers, usage_set);
        let max_cooldown = solution_cooldown.max(zombie_cooldown);
        if max_cooldown != f32::INFINITY {
            let mut duplicate_set: FxHashSet<Unlockable> = FxHashSet::with_capacity_and_hasher(4, BuildHasherDefault::default());
            for (i, plant) in self.plants.iter().copied().enumerate() {
                for plant_2 in self.plants.iter().copied().skip(i + 1) {
                    if plant == plant_2 {
                        duplicate_set.insert(plant);
                    }
                }
            }
            
            let plant_set: FxHashSet<Unlockable> = self.plants.iter().copied().collect();
            for plant in plant_set.iter().copied() {
                let (_, cooldown_mul) = modifiers.menu[plant as usize];
                let mut cooldown = COOLDOWN_TABLE[plant as usize] / (1. - usage_set[plant as usize]) * mul_from_u8(cooldown_mul);
                if plant != Unlockable::DoomShroom && !duplicate_set.contains(&plant) {
                    cooldown *= 0.5;
                }
                usage_set[plant as usize] = cooldown / max_cooldown; // TODO: this math feels unsound, i was very out of it while writing this
            }
            
            1. / (1. / zombie_cooldown - 1. / max_cooldown)
        } else {
            zombie_cooldown
        }
    }
    
    fn compute_solution_cooldown(&self, modifiers: &LevelPlants, usage_set: &[f32]) -> f32 {
        let mut max_cooldown  = 0f32;
        let mut slowest_plant = Unlockable::Peashooter;
        let mut plant_set: FxHashSet<Unlockable> = FxHashSet::with_capacity_and_hasher(4, BuildHasherDefault::default());
        for plant in self.plants.iter().copied() {
            let (_, cooldown_mul) = modifiers.menu[plant as usize];
            let mut cooldown = COOLDOWN_TABLE[plant as usize] / (1. - usage_set[plant as usize]);
            cooldown *= mul_from_u8(cooldown_mul);
            if plant != Unlockable::DoomShroom && !plant_set.contains(&plant) {
                cooldown *= 0.5;
            }
            if cooldown > max_cooldown {
                slowest_plant = plant;
                max_cooldown = cooldown;
            }
            plant_set.insert(plant);
        }
        if slowest_plant != Unlockable::SmallPuff && plant_set.contains(&Unlockable::SmallPuff) {
            max_cooldown /= 3.;
        }
        
        if let Some(uses) = self.uses {
            max_cooldown / uses as f32
        } else {
            0.
        }
    }
}

type Solutions = Box<[Box<[Unlockable]>]>;

impl RandomisationData {
    pub fn no_restrictions(seed: u64, meta: &IL2CppDumper, fuse_data: &FxHashMap<u32,[u32;2]>) -> Self {
        let plant_ids     = Self::get_plant_ids(meta);
        let level_order   = Self::randomise_level_order_no_restrictions(seed);
        let plant_order   = Self::randomise_plant_order_no_restrictions(seed);
        let mut weights   = Vec::new();
        let mut freqs     = Vec::new();
        let mut firerates = Vec::new();
        let mut cooldowns = Vec::new();
        let mut costs     = Vec::new();
        let mut spawns    = Vec::new();
        
        weights.push(vec![1, 0, 0, 0]);
        freqs.push(104f32.to_ne_bytes().to_vec());
        firerates.push(vec![0; plant_ids.len()]);
        cooldowns.push(vec![0; 2]);
        costs.push(vec![0; 2]);
        spawns.push(vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        
        for (level_true_idx, level_idx) in (2..=45).zip(level_order.iter().skip(1)) {
            weights.push(
                Self::randomise_weights_no_restrictions(
                    seed ^ hash_str(&level_true_idx.to_string()),
                )
            );
            firerates.push(
                Self::randomise_firerates_no_restrictions(
                    seed ^ hash_str(&level_true_idx.to_string()),
                    &plant_ids,
                    fuse_data,
                )
            );
            cooldowns.push(
                Self::randomise_cooldowns_no_restrictions(
                    seed ^ hash_str(&level_true_idx.to_string()),
                )
            );
            costs.push(
                Self::randomise_costs_no_restrictions(
                    seed ^ hash_str(&level_true_idx.to_string()),
                )
            );
            spawns.push(
                Self::randomise_spawns_no_restrictions(
                    seed ^ hash_str(&level_true_idx.to_string()),
                    *level_idx as usize,
                    level_true_idx,
                )
            );
            let data = Self::compute_zombie_freq_data_bytes(&spawns[level_true_idx - 1], &weights[level_true_idx - 1], *level_idx as usize).unwrap();
            freqs.push(data.totals);
        }
        
        let sound_seeds = Self::randomise_sounds(seed ^ hash_str("Sounds"));
        
        Self {
            level_order,
            plant_order,
            weights:     Some(weights),
            firerates:   Some(firerates),
            cooldowns:   Some(cooldowns),
            costs:       Some(costs),
            spawns:      Some(spawns),
            freqs:       Some(freqs),
            sound_seeds: Some(sound_seeds),
            restrictions_data: None,
        }
    }
    
    fn get_plant_ids(meta: &IL2CppDumper) -> Vec<u32> {
        let mut enum_variants: FxHashMap<String, u64> = HashMap::with_capacity_and_hasher(16384, BuildHasherDefault::default());
        let mut plant_ids: Vec<u32> = Vec::with_capacity(384);
        
        meta.get_enum_variants(&mut enum_variants);
        
        for (name, val) in enum_variants {
            if name.starts_with("PlantType::") && val as i64 >= 0 {
                plant_ids.push(val as u32);
            }
        }
        
        plant_ids.sort_unstable();
        
        plant_ids
    }
    
    fn weight_curve(int: u32) -> f64 {
        let num = int as f64 / u32::MAX as f64;
        ((64. / 15. * num - 32. / 5.) * num + 62. / 15.) * num - 1.
    }
    
    fn xor_bit_in_bitfield(bit: usize, bitfield: &mut [u8]) {
        bitfield[bit >> 3] ^= 1 << (bit & 7) as u8;
    }
    
    fn randomise_weights_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("zombie_weights")));
        let mut ret = Vec::with_capacity(ZOMBIE_DATA.get().unwrap().len() * 4);
        
        for zombie in ZOMBIE_DATA.get().unwrap() {
            let weight_mul = 10f64.powf(Self::weight_curve(rng.next_u32()));
            for byte in ((weight_mul * zombie.default_weight as f64).round() as i32).to_le_bytes() {
                ret.push(byte);
            }
        }
        
        ret
    }
    
    fn randomise_firerates_no_restrictions(seed: u64, plant_ids: &[u32], fuse_data: &FxHashMap<u32,[u32;2]>) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("plant_firerates")));
        
        let mut ret = vec![0u8; plant_ids.len()];
        for bytes in ret.chunks_exact_mut(8) {
            let val = rng.next_u64();
            let rand_bytes = val.to_le_bytes();
            for (byte, rbyte) in bytes.iter_mut().zip(rand_bytes.iter()) {
                *byte = *rbyte;
            }
        }
        let val = rng.next_u64();
        let remainder = ret.chunks_exact_mut(8).into_remainder();
        let rand_bytes = val.to_le_bytes();
        for (byte, rbyte) in remainder.iter_mut().zip(rand_bytes.iter()) {
            *byte = *rbyte;
        }
        
        Self::set_fusion_firerates(&mut ret, plant_ids, fuse_data);
        
        ret
    }
    
    fn randomise_cooldowns_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("plant_cooldowns")));
        
        let mut ret = vec![0u8; 48];
        for bytes in ret.chunks_exact_mut(8) {
            let val = rng.next_u64();
            let rand_bytes = val.to_le_bytes();
            for (byte, rbyte) in bytes.iter_mut().zip(rand_bytes.iter()) {
                *byte = *rbyte;
            }
        }
        ret[1] = u8::min(ret[1], 128);
        
        ret
    }
    
    fn randomise_costs_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("plant_costs")));
        
        let mut ret = vec![0u8; 48];
        for bytes in ret.chunks_exact_mut(8) {
            let val = rng.next_u64();
            let rand_bytes = val.to_le_bytes();
            for (byte, rbyte) in bytes.iter_mut().zip(rand_bytes.iter()) {
                *byte = *rbyte;
            }
        }
        ret[1] = u8::min(ret[1], 128);
        
        ret
    }
    
    fn randomise_spawns_no_restrictions(seed: u64, level_idx: usize, level_true_idx: usize) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("zombie_spawns")));
        let mut ret = vec![0u8; 16];
        let level = &LEVEL_DATA.get().unwrap()[level_idx - 1];
        
        for zombie_type in &level.default_zombie_ids {
            Self::xor_bit_in_bitfield(*zombie_type as usize, &mut ret);
        }
        
        for (i, zombie) in ZOMBIE_DATA.get().unwrap().iter().enumerate() {
            if let ZombieLanes::Water = zombie.allowed_lanes {
                match level.level_type {
                    LevelType::Pool |
                    LevelType::Fog => {}
                    _ => continue
                }
            }
            if (zombie.flags.contains(ZombieFlags::IS_ODYSSEY) && level_true_idx <= 30) || zombie.flags.contains(ZombieFlags::IS_BANNED) {
                continue;
            }
            
            let val = rng.next_u32();
            if (ret[i >> 8] & 1 << (i & 7) as u8) == 0 {
                if (!zombie.flags.contains(ZombieFlags::IS_ODYSSEY) && val >= u32::MAX / 20) ||
                    (zombie.flags.contains(ZombieFlags::IS_ODYSSEY) && val >= u32::MAX / 60) {
                    continue;
                }
            } else if val >= u32::MAX / 10 {
                continue;
            }
            
            if zombie.default_weight != 0 {
                Self::xor_bit_in_bitfield(i, &mut ret);
            }
        }
        
        ret[0] |= 1;
        
        ret
    }
    
    fn randomise_level_order_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("level_order")));
        
        let mut ret = vec![0u8; 45];
        let mut level_vec = Vec::with_capacity(45);
        
        for i in 1..=45 {
            level_vec.push((i, rng.next_u32()));
        }
        
        level_vec[0].1  = 0;
        
        level_vec.sort_by_key(|(_idx, val)| *val);
        
        for (i, (level_idx, _val)) in level_vec.iter().enumerate() {
            ret[i] = *level_idx as u8;
        }
        
        ret
    }
    
    fn randomise_plant_order_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("level_order")));
        
        let mut ret = vec![0u8; 48];
        let mut level_vec = Vec::with_capacity(45);
        
        level_vec.push((0, 0));
        level_vec.push((1, 0));
        
        for i in 2..41 {
            level_vec.push((i, rng.next_u32() >> 1));
        }
        
        level_vec.sort_by_key(|(_idx, val)| *val);
        
        for (i, (level_idx, _val)) in level_vec.iter().enumerate() {
            ret[i] = *level_idx as u8;
        }
        
        for i in ret.iter_mut().skip(41) {
            *i = 0xFF
        }
        
        ret
    }
    
    fn set_fusion_firerates(firerates: &mut [u8], plant_ids: &[u32], fuse_data: &FxHashMap<u32,[u32;2]>) {
        let mut plant_lookup: FxHashMap<u32,u32> = HashMap::with_capacity_and_hasher(plant_ids.len(), BuildHasherDefault::default());
        
        for (i, plant_id) in plant_ids.iter().enumerate() {
            plant_lookup.insert(*plant_id, i as u32);
        }
        
        fn get_fused_firerate(
            fuse_plants: [u32;2],
            plant_lookup: &FxHashMap<u32,u32>,
            fuse_data: &FxHashMap<u32,[u32; 2]>,
            firerates: &[u8],
            recursion: u32,
        ) -> u8 {
            let mut ret: i32 = 0;
            if recursion < 5 {
                for plant in fuse_plants {
                    if let Some(fuse_plants) = fuse_data.get(&plant) {
                        ret += get_fused_firerate(*fuse_plants, plant_lookup, fuse_data, firerates, recursion + 1) as i32;
                    } else {
                        ret += firerates[*plant_lookup.get(&plant).unwrap() as usize] as i32;
                    }
                }
            }
            
            (ret >> 1) as u8
        }
        
        for (i, plant_id) in plant_ids.iter().enumerate() {
            if let Some(fuse_plants) = fuse_data.get(plant_id) {
                firerates[i] = get_fused_firerate(*fuse_plants, &plant_lookup, fuse_data, firerates, 0)
            }
        }
    }
    
    fn randomise_sounds(seed: u64) -> Vec<u64> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("random_sounds")));
        let mut ret = Vec::with_capacity(45);
        for _ in 0..45 {
            ret.push(rng.next_u64());
        }
        ret
    }
    
    
    
    
    
    
    fn weight_curve_inverse(x: f64) -> f64 {
        let x2  = x*x;
        ((0.43080993081*x2-0.835882783883)*x2+0.905073260073)*x+0.5 //a crude approximation of the inverse of the input polynomial
    }
    
    fn get_zombie_map() -> FxHashMap<ZombieType, u32> {
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        
        let mut ret = HashMap::with_capacity_and_hasher(128, BuildHasherDefault::default());
        
        for (i, zombie) in zombie_data.iter().enumerate() {
            ret.insert(zombie.zombie_type, i as u32);
        }
        
        ret
    }
    
    fn get_plant_map_and_ids(meta: &IL2CppDumper) -> (FxHashMap<String, u32>, Vec<u32>, FxHashMap<u32, u32>) {
        let mut enum_variants: FxHashMap<String, u64> = HashMap::with_capacity_and_hasher(16384, BuildHasherDefault::default());
        let mut plant_map: FxHashMap<String, u32> = HashMap::with_capacity_and_hasher(384, BuildHasherDefault::default());
        let mut rev_map: FxHashMap<u32, u32> = HashMap::with_capacity_and_hasher(384, BuildHasherDefault::default());
        let mut plant_ids: Vec<u32> = Vec::with_capacity(384);
        
        meta.get_enum_variants(&mut enum_variants);
        
        for (name, val) in enum_variants {
            if name.starts_with("PlantType::") && val as i64 >= 0 {
                plant_ids.push(val as u32);
                plant_map.insert(name.strip_prefix("PlantType::").unwrap().to_owned(), val as u32);
            }
        }
        
        plant_ids.sort_unstable();
        
        for (i, plant_id) in plant_ids.iter().enumerate() {
            rev_map.insert(*plant_id, i as u32);
        }
        
        for val in plant_map.values_mut() {
            *val = *rev_map.get(val).unwrap();
        }
        
        (plant_map, plant_ids, rev_map)
    }
    
    fn randomise_plant_attrs(&mut self, meta: &IL2CppDumper, fuse_data: &FxHashMap<u32,[u32;2]>, seed: u64) {
        let (plant_map, plant_ids, rev_map) = Self::get_plant_map_and_ids(meta);
        
        let mut cost_rng      = ChaCha8Rng::seed_from_u64(seed ^ hash_str("plant_cost"));
        let mut cooldowns_rng = ChaCha8Rng::seed_from_u64(seed ^ hash_str("plant_cooldowns"));
        let mut firerates_rng = ChaCha8Rng::seed_from_u64(seed ^ hash_str("plant_firerates"));
        let restrictions_data = self.restrictions_data.as_mut().unwrap();
        
        let mut non_fused_ids: FxHashSet<u32> = plant_ids.iter().copied().collect();
        for k in fuse_data.keys() {
            non_fused_ids.remove(k);
        }
        
        for level_idx in 2..=45 {
            let mut cooldowns: Vec<(u32, u8)> = Vec::with_capacity(48);
            let mut costs:     Vec<(u32, u8)> = Vec::with_capacity(48);
            let mut firerates: Vec<u8> = vec![0; plant_ids.len()];
            for i in 0..48 {
                let byte = (Self::weight_curve(i * 0x572_620A) * 127.5 + 127.5).round() as u8; //0x572_620A is 2^32 / 47
                cooldowns.push((cooldowns_rng.next_u32(), if self.cooldowns.is_some() {byte} else {0x80}));
                costs.push((cost_rng.next_u32(), if self.costs.is_some() {byte} else {0x80}));
            }
            
            for i in non_fused_ids.iter() {
                firerates[*rev_map.get(i).unwrap() as usize] = (if self.firerates.is_some() {firerates_rng.next_u32() >> 24} else {0x80}) as u8;
            }
            
            cooldowns.sort_by_key(|(key, _)| *key);
            costs.sort_by_key(|(key, _)| *key);
            
            let mut menu: Vec<(u8, u8)> = cooldowns
                .iter()
                .zip(costs.iter())
                .map(|((_, cd), (_, cs))| (*cd, *cs))
                .collect();
            
            menu[1].0 = menu[1].0.min(0x80);
            menu[1].1 = menu[1].1.min(0x80);
            
            Self::set_fusion_firerates(&mut firerates, &plant_ids, fuse_data);
            
            restrictions_data.level_plants.insert(level_idx, LevelPlants {
                menu,
                all: firerates,
            });
        }
        
        restrictions_data.plant_map = plant_map;
    }
    
    fn get_solutions_1() -> FxHashMap<Problem, Solutions> {
        [
            (Problem::Water1, vec![
                vec![
                    Unlockable::Tanglekelp,
                ].into_boxed_slice(),
                vec![
                    Unlockable::SeaShroom,
                ].into_boxed_slice(),
                vec![
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::ThreePeater,
                ].into_boxed_slice(),
                vec![
                    Unlockable::StarFruit,
                ].into_boxed_slice(),
                vec![ //gloom should be safe on a 1 flag as long as you can get 2 up quickly
                    Unlockable::FumeShroom,
                    Unlockable::EndoFlame,
                ].into_boxed_slice(),
                vec![ //same here
                    Unlockable::FumeShroom,
                    Unlockable::GloomShroom,
                ].into_boxed_slice(),
                //vec![ //present cooldown is too long these days for this to be practical
                //    Unlockable::Present,
                //].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::Roof, vec![
                vec![
                    Unlockable::Pot,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::HighHealth, vec![
                vec![
                    Unlockable::HypnoShroom,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Chomper,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Squash,
                ].into_boxed_slice(),
                vec![
                    Unlockable::CherryBomb,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Jalapeno,
                ].into_boxed_slice(),
                vec![
                    Unlockable::TorchWood,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Plantern,
                    Unlockable::StarFruit,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Melonpult,
                ].into_boxed_slice(),
                //vec![ //doom cooldown is too long for this to be practical
                //    Unlockable::DoomShroom,
                //].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::Balloon, vec![
                vec![
                    Unlockable::Cactus,
                ].into_boxed_slice(),
                vec![
                    Unlockable::LilyPad, //cattail
                ].into_boxed_slice(),
                vec![
                    Unlockable::Blover,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::NoPuff, vec![
                vec![
                    Unlockable::SmallPuff,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
        ].into_iter().collect()
    }
    
    fn get_solutions_2() -> FxHashMap<Problem, Solutions> {
        [
            (Problem::Water2, vec![
                vec![
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::ThreePeater,
                ].into_boxed_slice(),
                vec![
                    Unlockable::StarFruit,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::ReallyHighHealth, vec![
                vec![
                    Unlockable::Chomper,
                ].into_boxed_slice(),
                vec![
                    Unlockable::HypnoShroom,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::BalloonWater, vec![
                vec![
                    Unlockable::Cactus, //this is just here so that this option can have increased weight if cactus is selected
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Cactus,
                    Unlockable::SeaShroom,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Cactus,
                    Unlockable::StarFruit,
                ].into_boxed_slice(),
                vec![
                    Unlockable::LilyPad, //cattail
                ].into_boxed_slice(),
                vec![
                    Unlockable::Blover,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::Snorkle, vec![
                vec![
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Tanglekelp,
                    Unlockable::Squash,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Tanglekelp,
                    Unlockable::Jalapeno,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
        ].into_iter().collect()
    }
    
    fn get_solutions_3() -> FxHashMap<Problem, Solutions> {
        [
            (Problem::Water34, vec![
                vec![
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::ThreePeater,
                    Unlockable::Tanglekelp,
                    Unlockable::TorchWood,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Cornpult,
                    Unlockable::SeaShroom, //earlygame because cob cannon is bad in earlygame, plus so you can put sun on water
                    Unlockable::EndoFlame,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::DarkMicheal, vec![
                vec![
                    Unlockable::WallNut,
                    Unlockable::TallNut,
                ].into_boxed_slice(),
                vec![
                    Unlockable::WallNut,
                    Unlockable::EndoFlame,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::Kirov, vec![
                vec![
                    Unlockable::Cactus,
                    Unlockable::Plantern,
                    Unlockable::LilyPad,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Cactus,
                    Unlockable::StarFruit,
                ].into_boxed_slice(),
                vec![
                    Unlockable::LilyPad,
                    Unlockable::CattailPlant, //we include cattail/endoflame here because you probably want a lot of them to counter kirov
                ].into_boxed_slice(),
                vec![
                    Unlockable::LilyPad,
                    Unlockable::EndoFlame, //also I am aware this can't counter land balloons, too bad!
                ].into_boxed_slice(),
            ].into_boxed_slice()),
            (Problem::Gargantaur, vec![
                vec![
                    Unlockable::CherryBomb,
                ].into_boxed_slice(),
                vec![
                    Unlockable::TorchWood,
                    Unlockable::ThreePeater,
                ].into_boxed_slice(),
                vec![
                    Unlockable::TorchWood,
                    Unlockable::Jalapeno,
                ].into_boxed_slice(),
                vec![
                    Unlockable::Cornpult,
                    Unlockable::EndoFlame,
                    Unlockable::Jalapeno,
                ].into_boxed_slice(),
                vec![
                    Unlockable::StarFruit,
                    Unlockable::Plantern,
                ].into_boxed_slice(),
            ].into_boxed_slice()),
        ].into_iter().collect()
    }
    
    fn get_solutions_all() -> FxHashMap<Problem, Solutions> {
        let iter = Self::get_solutions_1().into_iter()
            .chain(Self::get_solutions_2())
            .chain(Self::get_solutions_3());
        iter.collect()
    }
    
    fn compute_zombie_freq_data_bytes(spawns: &[u8], weights: &[u8], level: usize) -> Option<FrequencyData> {
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        
        let mut spawn_vec: Vec<(u32, u32, u32)> = Vec::with_capacity(weights.len() >> 2);
        for (i, bytes) in weights.chunks_exact(4).enumerate() {
            if spawns[i >> 3] & (1 << (i & 7)) != 0 {
                spawn_vec.push((i as u32, u32::from_le_bytes(bytes.try_into().unwrap()), zombie_data[i].default_points));
            }
        }
        
        spawn_vec.sort_by_key(|(_, _, points)| *points);
        
        Self::compute_zombie_freq_data(&spawn_vec, level)
    }
    
    fn compute_zombie_freq_data(spawn_vec: &[(u32, u32, u32)], level: usize) -> Option<FrequencyData> {
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        let level_data = LEVEL_DATA.get().unwrap();
        
        let mut spawn_vec_pre_10 = spawn_vec.to_vec();
        let mut pre_10_map = Vec::from_iter(0..spawn_vec.len());
        
        for (i, (idx, _, _)) in spawn_vec.iter().enumerate().rev() {
            if zombie_data[*idx as usize].flags.contains(ZombieFlags::IS_ELITE) {
                spawn_vec_pre_10.remove(i);
                pre_10_map.remove(i);
            }
        }
        
        fn compute_freq_for_wave(spawn_vec: &[(u32, u32, u32)], wave: isize) -> Vec<f64> {
            let mut wavepoints_max = 0;
            
            for (_, _, points) in spawn_vec {
                wavepoints_max = isize::max(*points as isize, wavepoints_max);
            }
            
            assert_ne!(wavepoints_max, 0);
            
            let mut spawns_lut: Vec<Vec<f64>> = Vec::with_capacity(wavepoints_max as usize - 1);
            
            for i in 1 ..= wavepoints_max {
                let mut sum = 0f64;
                let mut vec: Vec<f64> = Vec::new();
                for (_, weight, _) in spawn_vec.iter().take_while(|(_, _, points)| *points as isize <= i) {
                    sum += *weight as f64;
                    vec.push(*weight as f64);
                }
                
                for weight in vec.iter_mut() {
                    *weight /= sum;
                }
                spawns_lut.push(vec);
            }
            
            let total_wavepoints = (wave as usize * 5 / 3) * if wave % 10 == 0 {2} else {1};
            let mut zombie_odds: Vec<f64> = vec![0f64; spawn_vec.len()];
            let mut wavepoint_odds_array = vec![0f64; total_wavepoints];
            wavepoint_odds_array[total_wavepoints - 1] = 1f64;
            
            for remaining_points in (1..=total_wavepoints).rev() {
                let chance_1 = wavepoint_odds_array[remaining_points - 1];
                for (chance_2, ((_, _, choice_wavepoints), choice_odds)) in
                    spawns_lut[usize::min(remaining_points, spawns_lut.len()) - 1]
                    .iter()
                    .zip(spawn_vec.iter().zip(zombie_odds.iter_mut()))
                {
                    let chance = chance_1 * *chance_2;
                    *choice_odds += chance;
                    if remaining_points > *choice_wavepoints as usize {
                        wavepoint_odds_array[remaining_points - *choice_wavepoints as usize - 1] += chance;
                    }
                }
            }
            
            zombie_odds
        }
        
        let wave_max = level_data[level - 1].flags? as isize * 10;
        let mut freq_array = vec![f32::NAN; wave_max as usize * spawn_vec.len()];
        
        for wave in 1 ..= wave_max {
            let spawn_data = if wave < 10 {&spawn_vec_pre_10} else {spawn_vec};
            let zombie_freq = compute_freq_for_wave(spawn_data, wave);
            if wave < 10 {
                let off = spawn_vec.len() * (wave as usize - 1);
                for dst in freq_array
                    .iter_mut()
                    .skip(spawn_vec.len() * (wave as usize - 1))
                    .take(spawn_vec.len()) {
                    *dst = 0f32;
                }
                for (src, idx) in zombie_freq.iter().zip(pre_10_map.iter()) {
                    freq_array[off + *idx] = *src as f32;
                }
            } else {
                for (dst, src) in freq_array
                    .iter_mut()
                    .skip(spawn_vec.len() * (wave as usize - 1))
                    .take(spawn_vec.len())
                    .zip(zombie_freq.iter()) {
                    *dst = *src as f32;
                }
            }
        }
        
        let mut max_frequency = HashMap::default();
        let mut first_flag_totals = HashMap::default();
        let mut first_2_flag_max = HashMap::default();
        let mut first_wave_occurence_avgs = HashMap::default();
        let mut totals = vec![0x3F; zombie_data.len() * 4];
        
        for (i, (id, _, _)) in spawn_vec.iter().enumerate() {
            let mut max_freq      = 0f32;
            let mut max_wave      = 1;
            let mut total_freq    = 0f32;
            let mut total_freq_ff = 0f32;
            let mut max_freq_f2f  = 0f32;
            let mut first_wave    = 0;
            for (freq, j) in freq_array.iter().skip(i).step_by(spawn_vec.len()).zip(1..) {
                let freq_mul = if j % 10 == 0 {0.5} else {1.0};
                if *freq * freq_mul > max_freq {
                    max_freq = *freq;
                    max_wave = j;
                    if j < 20 {
                        max_freq_f2f = *freq;
                    }
                }
                if j < 10 {
                    total_freq_ff += *freq;
                }
                total_freq += *freq;
                if total_freq < 0.5 {
                    first_wave = j + 1;
                }
            }
            first_wave = u32::max(first_wave, wave_max as u32);
            max_frequency.insert(*id, (max_freq, max_wave));
            first_flag_totals.insert(*id, total_freq_ff);
            first_2_flag_max.insert(*id, max_freq_f2f);
            first_wave_occurence_avgs.insert(*id, first_wave);
            for (dst, src) in totals[*id as usize * 4 .. 4 + *id as usize * 4].iter_mut().zip((total_freq+1.).to_le_bytes()) {
                *dst = src;
            }
        }
        
        Some(FrequencyData {
            raw_averages: freq_array,
            max_frequency,
            first_flag_totals,
            first_2_flag_max,
            first_wave_occurence_avgs,
            totals,
        })
    }
    
    pub fn compute_zombie_freq_data_cached(&mut self, level_spawns: &[(u32,u32)], level: usize) -> Option<FrequencyData> {
        let key = FrequencyCacheKey {
            spawns: level_spawns.into(),
            level,
        };
        
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        
        let mut spawn_vec: Vec<(u32, u32, u32)> = Vec::with_capacity(level_spawns.len());
        for (idx, weight) in level_spawns.iter() {
            spawn_vec.push((*idx, *weight, zombie_data[*idx as usize].default_points));
        }
        
        spawn_vec.sort_by_key(|(_, _, points)| *points);
        
        if let Some(restrictions_data) = &mut self.restrictions_data.as_mut() {
            let frequency_cache = &mut restrictions_data.frequency_cache;
            if let Some(entry) = frequency_cache.get(&key) {
                return Some(entry.clone());
            }
            
            if let Some(freq_data) = Self::compute_zombie_freq_data(&spawn_vec, level) {
                frequency_cache.insert(key, freq_data.clone());
                Some(freq_data)
            } else {
                None
            }
        } else {
            Self::compute_zombie_freq_data(&spawn_vec, level)
        }
    }
    
    fn is_any_solution_satisfied(
        &mut self,
        solutions: &Solutions,
        level: &LevelData,
        used_solutions: &mut FxHashMap<Solutions, u32>,
        importance: u32
    ) -> bool {
        let unlocked_plants = if let Some(conveyor_plants) = &level.conveyor_plants {
            conveyor_plants
        } else {
            &self.restrictions_data.as_ref().unwrap().unlocked_plants
        };
        
        let mut vec: Vec<Box<[Unlockable]>> = Vec::with_capacity(12); //vec is necessary to prevent mutable + immutable borrow
        let mut solution_found = false;
        
        'solution_loop: for solution in solutions {
            for unlockable in solution {
                if !unlocked_plants.contains(unlockable) {
                    continue 'solution_loop;
                }
            }
            vec.push(solution.clone());
            solution_found = true;
        }
        
        if solution_found {
            used_solutions.entry(vec.into_boxed_slice()).and_modify(|x| *x += importance).or_insert(importance);
        }
        
        solution_found
    }
    
    #[allow(dead_code)]
    fn get_zombie_solutions(&self, freq_data: &FrequencyData, zombie_idx: u32) -> SmallVec<[ProblemData; 1]> {
        let mut out_vec = SmallVec::new();
        static SOLUTION_TABLE: OnceLock<FxHashMap<ZombieFlags, Box<[SolutionEntry]>>> = OnceLock::new();
        static ZOMBIE_SOLUTION_TABLE: OnceLock<FxHashMap<ZombieType, Box<[SolutionEntry]>>> = OnceLock::new();
        let solution_table = &SOLUTION_TABLE.get_or_init(|| {
            let mut solution_table: FxHashMap<ZombieFlags, Box<[SolutionEntry]>> = [
                (ZombieFlags::HIGH_HEALTH, vec![
                    SolutionEntry {
                        plants: [Unlockable::Squash].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Jalapeno].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::CherryBomb].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::HypnoShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Chomper].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::DoomShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::DoomShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::WallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::WallNut].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Pumpkin].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::EVIL_DEATH, vec![
                    SolutionEntry {
                        plants: [Unlockable::Squash].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Jalapeno].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::CherryBomb].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::HypnoShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Chomper].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::DoomShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::DoomShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::WallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::V_HIGH_HEALTH, vec![
                    SolutionEntry {
                        plants: [Unlockable::HypnoShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Chomper].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::WallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::WallNut].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Pumpkin].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::FLIES, vec![
                    !SolutionEntry {
                        plants: [Unlockable::Squash].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Jalapeno].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::CherryBomb].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::DoomShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [
                            Unlockable::DoomShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::HypnoShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::Chomper].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [
                            Unlockable::WallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::WallNut].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::Pumpkin].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Cactus,
                            Unlockable::StarFruit,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Cactus,
                            Unlockable::Plantern,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Cactus,
                            Unlockable::DoomShroom,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::GARG_TYPE, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::WallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Squash].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Jalapeno].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::CherryBomb].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Chomper].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::DoomShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::DoomShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(2),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::IS_VEHICLE, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Caltrop,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(4),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Caltrop].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Caltrop,
                            Unlockable::ThreePeater,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::IS_METAL, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Magnetshroom,
                            Unlockable::Blover,
                        ].into_iter().collect(),
                        uses: Some(4),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [Unlockable::Magnetshroom].into_iter().collect(),
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Magnetshroom,
                            Unlockable::Plantern,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieFlags::DOES_NOT_EAT, vec![
                    !SolutionEntry {
                        plants: [
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::HypnoShroom].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::WallNut].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [Unlockable::Pumpkin].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
            ].into_iter().collect();
            
            for val in solution_table.values_mut() {
                for solution in val {
                    solution.plants.sort_unstable_by_key(|x| *x as isize);
                }
            }
            
            solution_table
        });
        
        let zombie_solution_table = &ZOMBIE_SOLUTION_TABLE.get_or_init(|| {
            let mut solution_table: FxHashMap<ZombieType, Box<[SolutionEntry]>> = [
                (ZombieType::ElitePaperZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Caltrop,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: Some(6), //total guess
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::DancePolZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::TallNut,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::EndoFlame,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: None, //total guess
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [
                            Unlockable::Chomper,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::PogoZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cabbagepult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cornpult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Garlic,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::SuperPogoZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cabbagepult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cornpult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Garlic,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::JackboxJumpZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cabbagepult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Cornpult,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                            Unlockable::Garlic,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::SnowZombie, vec![
                    !SolutionEntry {
                        plants: [
                            Unlockable::Chomper,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                    !SolutionEntry {
                        plants: [
                            Unlockable::Squash,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::MachineNutZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::SuperMachineNutZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Umbrellaleaf,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::BalloonZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::Blover,
                        ].into_iter().collect(),
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::SuperCherryShooterZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::Pumpkin,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::CherryPaperZombie, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::Pumpkin,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
                (ZombieType::CherryPaperZ95, vec![
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::Pumpkin,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                    SolutionEntry {
                        plants: [
                            Unlockable::CherryBomb,
                            Unlockable::WallNut,
                        ].into_iter().collect(),
                        uses: None,
                        ..Default::default()
                    },
                ].into_boxed_slice()),
            ].into_iter().collect();
            
            for val in solution_table.values_mut() {
                for solution in val {
                    solution.plants.sort_unstable_by_key(|x| *x as isize);
                }
            }
            
            solution_table
        });
        
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        let zombie = &zombie_data[zombie_idx as usize];
        let flags = zombie.flags;
        let problem_flags = flags.intersection(
            ZombieFlags::HIGH_HEALTH |
            ZombieFlags::V_HIGH_HEALTH |
            ZombieFlags::FLIES |
            ZombieFlags::GARG_TYPE |
            ZombieFlags::EVIL_DEATH
        );
        
        let attrib_flags = flags & !problem_flags;
        
        if problem_flags.is_empty() {
            return out_vec;
        }
        
        for flag in problem_flags {
            let mut solutions: FxHashSet<SolutionEntry> = FxHashSet::with_capacity_and_hasher(64, BuildHasherDefault::default());
            let mut not_solutions: Vec<SolutionEntry> = Vec::with_capacity(64);
            if let Some(problem_solutions) = solution_table.get(&flag) {
                for entry in problem_solutions {
                    if !entry.negate {
                        solutions.insert(entry.clone());
                    } else {
                        not_solutions.push(entry.clone());
                    }
                }
            } else {
                unreachable!()
            }
            for flag in attrib_flags {
                if let Some(problem_solutions) = solution_table.get(&flag) {
                    for entry in problem_solutions {
                        if !entry.negate {
                            solutions.insert(entry.clone());
                        } else {
                            not_solutions.push(entry.clone());
                        }
                    }
                }
            }
            if let Some(problem_solutions) = zombie_solution_table.get(&zombie.zombie_type) {
                for entry in problem_solutions {
                    if !entry.negate {
                        solutions.insert(entry.clone());
                    } else {
                        not_solutions.push(entry.clone());
                    }
                }
            }
            
            for entry in not_solutions {
                solutions.remove(&entry);
            }
            
            let freq = match flag {
                ZombieFlags::EVIL_DEATH => freq_data.max_frequency.get(&zombie_idx).unwrap().0
                    .max(*freq_data.first_flag_totals.get(&zombie_idx).unwrap()),
                _ => *freq_data.first_2_flag_max.get(&zombie_idx).unwrap(),
            };
            
            let solutions: FxHashSet<SolutionEntry> = solutions.into_iter().collect();
            
            out_vec.push(ProblemData { zombie_idx, orig_freq: freq, freq, solutions });
        }
        
        out_vec
    }
    
    fn is_level_possible(&mut self, level_idx: u32, level_true_idx: u32, seed: u64) -> Result<(),Vec<ImpossibleReason>> {
        static FIREPOWER_SOLUTIONS: OnceLock<FxHashMap<u8, Box<[SolutionEntry]>>> = OnceLock::new();
        
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("more_plant_stuff")) ^ level_idx as u64);
        let mut ret = Vec::new();
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        let level = &LEVEL_DATA.get().unwrap()[level_idx as usize - 1];
        let solutions = Self::get_solutions_all();
        
        let mut used_solutions: FxHashMap<Solutions, u32> = HashMap::with_capacity_and_hasher(64, BuildHasherDefault::default());
        
        {
            //let basic_dps = vec![ //makes sure there is a single okay firepower plant
            //    vec![Unlockable::Peashooter].into_boxed_slice(),
            //    vec![Unlockable::SmallPuff].into_boxed_slice(),
            //    vec![Unlockable::FumeShroom].into_boxed_slice(),
            //    vec![Unlockable::ScaredyShroom, Unlockable::DoomShroom].into_boxed_slice(),
            //    vec![Unlockable::ThreePeater].into_boxed_slice(),
            //    vec![Unlockable::Cactus, Unlockable::DoomShroom].into_boxed_slice(),
            //    vec![Unlockable::StarFruit].into_boxed_slice(),
            //    vec![Unlockable::Cabbagepult].into_boxed_slice(),
            //    vec![Unlockable::Melonpult].into_boxed_slice(),
            //].into_boxed_slice();
            //self.is_any_solution_satisfied(&basic_dps, level, &mut used_solutions, 3);
            
            let firepower_solutions = FIREPOWER_SOLUTIONS.get_or_init(|| {
                let mut firepower_solutions: FxHashMap<u8, Box<[SolutionEntry]>> = [
                    (1, vec![
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::SmallPuff,
                            ].into_iter().collect(),
                            night: true,
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::FumeShroom,
                            ].into_iter().collect(),
                            night: true,
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ScaredyShroom,
                            ].into_iter().collect(),
                            night: true,
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::StarFruit,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Cabbagepult,
                                Unlockable::Cornpult,
                            ].into_iter().collect(),
                            uses: None,
                            weight: 0.5,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Cabbagepult,
                                Unlockable::Garlic,
                            ].into_iter().collect(),
                            uses: None,
                            weight: 0.5,
                            ..Default::default()
                        },
                    ].into_boxed_slice()),
                    (2, vec![
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                                Unlockable::SmallPuff,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::SmallPuff,
                                Unlockable::StarFruit,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::FumeShroom,
                                Unlockable::ScaredyShroom,
                            ].into_iter().collect(),
                            night: true,
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                                Unlockable::Squash
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                                Unlockable::TorchWood,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Cactus,
                                Unlockable::Plantern,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Melonpult,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                    ].into_boxed_slice()),
                    (3, vec![
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                                Unlockable::CherryBomb,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                                Unlockable::TorchWood,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                                Unlockable::SmallPuff,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                                Unlockable::Squash
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                                Unlockable::TorchWood,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::StarFruit,
                                Unlockable::Plantern,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Cornpult,
                                Unlockable::EndoFlame,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Melonpult,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                    ].into_boxed_slice()),
                    (4, vec![
                        SolutionEntry {
                            plants: [
                                Unlockable::Peashooter,
                                Unlockable::TorchWood,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::ThreePeater,
                                Unlockable::TorchWood,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::StarFruit,
                                Unlockable::Plantern,
                                Unlockable::Pumpkin,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Cornpult,
                                Unlockable::EndoFlame,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Melonpult,
                                Unlockable::Jalapeno,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                        SolutionEntry {
                            plants: [
                                Unlockable::Melonpult,
                                Unlockable::Cabbagepult,
                                Unlockable::Umbrellaleaf,
                            ].into_iter().collect(),
                            uses: None,
                            ..Default::default()
                        },
                    ].into_boxed_slice()),
                ].into_iter().collect();
                
                for val in firepower_solutions.values_mut() {
                    for solution in val {
                        solution.plants.sort_unstable_by_key(|x| *x as isize);
                    }
                }
                
                firepower_solutions
            });
            
            let solutions = firepower_solutions.get(&level.flags.unwrap_or(4)).unwrap();
            let mut options: SmallVec<[SolutionEntry; 8]> = SmallVec::new();
            
            let unlocked_plants = if let Some(conveyor_plants) = &level.conveyor_plants {
                conveyor_plants
            } else {
                &self.restrictions_data.as_ref().unwrap().unlocked_plants
            };
            
            for solution in solutions {
                if !solution.night || level.level_type == LevelType::Night || level.level_type == LevelType::Fog {
                    if solution.plants.iter().all(|plant| unlocked_plants.contains(plant)) {
                        options.push(solution.clone());
                    }
                }
            }
            
            if options.is_empty() {
                ret.push(ImpossibleReason::FourFlag); //not necessarily the case, but who cares
            } else {
                let mut cumulative_weights: SmallVec<[f64; 8]> = SmallVec::new();
                let mut total_weight = 0f64;
                for solution in &options {
                    cumulative_weights.push(total_weight);
                    total_weight += solution.weight as f64;
                }
                assert_ne!(total_weight, 0f64);
                let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
                let idx = cumulative_weights.partition_point(|csum| *csum <= val);
                let solution = &options[idx - 1];
                used_solutions.insert(vec![solution.plants.iter().copied().collect()].into_boxed_slice(), 3);
            }
        }
        
        if level.conveyor_plants.is_none() {
            let flags = level.flags.unwrap();
            match level.level_type {
                LevelType::Pool |
                LevelType::Fog => {
                    match flags {
                        1 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water1).unwrap(), level, &mut used_solutions, 3)
                            && !(2..=4).contains(&(level_true_idx % 7)) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        2 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water2).unwrap(), level, &mut used_solutions, 3)
                            && !(2..=4).contains(&(level_true_idx % 7)) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        3 |
                        4 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water34).unwrap(), level, &mut used_solutions, 3) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        _ => unreachable!(),
                    }
                }
                LevelType::Roof => {
                    match flags {
                        1 => {}
                        _ => if !self.is_any_solution_satisfied(solutions.get(&Problem::Roof).unwrap(), level, &mut used_solutions, 3) {
                            ret.push(ImpossibleReason::NoPot);
                        }
                    }
                }
                _ => {}
            }
        }
        
        
        let spawns = self.restrictions_data.as_ref().unwrap().level_spawns.get(&(level_idx as u8)).unwrap().clone();
        let spawns_map: FxHashMap<u32, u32> = spawns.iter().map(|(k, v)| (*k, *v)).collect();
        let zombie_map = Self::get_zombie_map();
        let mut threshold_table = vec![999f32; zombie_data.len()];
        if let Some(spawn_data) = self.compute_zombie_freq_data_cached(&spawns, level_idx as usize) {
            let restrictions_data = self.restrictions_data.as_ref().unwrap();
            let plant_data = restrictions_data.level_plants.get(&(level_idx as u8)).unwrap();
            
            let unlocked_plants = if let Some(conveyor_plants) = &level.conveyor_plants {
                conveyor_plants
            } else {
                &self.restrictions_data.as_ref().unwrap().unlocked_plants
            };
            
            let mut problem_vec: Vec<ProblemData> = spawns
                .iter()
                .flat_map(|(i, _)| self.get_zombie_solutions(&spawn_data, *i).into_iter())
                .map(|mut data| {
                    data.solutions = data.solutions
                        .into_iter()
                        .filter(|solution| {
                            solution.plants
                                .iter()
                                .all(|plant| unlocked_plants.contains(&plant))
                        })
                        .collect();
                    data
                })
                .collect();
            
            let mut solution_set: FxHashMap<SolutionEntry, (f64, u32)> = HashMap::with_capacity_and_hasher(512, BuildHasherDefault::default());
            let mut usage_array = [0f32; 41];
            let mut ldlist: Vec<(u32, Option<u32>)> = Vec::with_capacity(768);
            
            for _ in 0..5 {
                for (i, solution_data) in problem_vec.iter().enumerate() {
                    for entry in &solution_data.solutions {
                        let mut next: Option<u32> = None;
                        if let Some((f, e)) = solution_set.get_mut(entry) {
                            *f += solution_data.freq as f64;
                            next = Some(*e);
                            *e = ldlist.len() as u32;
                        } else {
                            solution_set.insert(entry.clone(), (solution_data.freq as f64, ldlist.len() as u32));
                        }
                        
                        ldlist.push((i as u32, next));
                    }
                }
                
                let mut total_weight = 0f64;
                let mut to_remove: FxHashSet<SolutionEntry> = HashSet::with_capacity_and_hasher(4, BuildHasherDefault::default());
                for (entry, (cumulative_weight, _)) in solution_set.iter_mut() {
                    let weight = cumulative_weight.min(4. / entry.compute_solution_cooldown(plant_data, &usage_array) as f64) * entry.weight as f64;
                    *cumulative_weight = total_weight;
                    total_weight += weight;
                    if weight == 0. {
                        to_remove.insert(entry.clone());
                    }
                }
                let solution_vec: Vec<(SolutionEntry, (f64, u32))> = solution_set.drain().filter(|(s, (_, _))| !to_remove.contains(s)).collect();
                
                if solution_vec.is_empty() {
                    break;
                }
                
                assert_ne!(total_weight, 0f64);
                let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
                let idx = solution_vec.partition_point(|(_, (csum, _))| *csum <= val);
                let (solution, (_, idx)) = &solution_vec[idx - 1];
                used_solutions.insert(vec![solution.plants.iter().copied().collect()].into_boxed_slice(), 1);
                let mut ldlist_idx = Some(*idx);
                let mut problem_list: SmallVec<[u32; 16]> = SmallVec::new();
                while ldlist_idx.is_some() {
                    let (problem_idx, next_idx) = ldlist[unsafe {ldlist_idx.unwrap_unchecked()} as usize];
                    problem_vec[problem_idx as usize].solutions.remove(solution);
                    problem_list.push(problem_idx);
                    ldlist_idx = next_idx;
                }
                
                while solution.compute_solution_cooldown(plant_data, &usage_array) != f32::INFINITY  && !problem_list.is_empty() {
                    let problem_idx_idx = ((rng.next_u32() as u64 * problem_list.len() as u64) >> 32) as usize;
                    let problem_idx = problem_list[problem_idx_idx];
                    let problem = &mut problem_vec[problem_idx as usize];
                    problem.freq = 3. / solution.compute_new_plant_utilization(plant_data, &mut usage_array, 3. / problem.freq);
                    if problem.freq == 0. {
                        problem_list.remove(problem_idx_idx);
                    } else {
                        break;
                    }
                }
                
                if problem_vec.is_empty() {
                    break;
                }
                
                ldlist.drain(..);
            }
            
            for problem in problem_vec {
                threshold_table[problem.zombie_idx as usize] = (problem.orig_freq - problem.freq).max(0.05);
            }
            
            {
                let (zombie_type, low_threshold, high_threshold) = (ZombieType::SnorkleZombie,0.1,0.45);
                let zombie_idx = *zombie_map.get(&zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type:?}\""));
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    let max_frequency = *max_frequency;
                    if max_frequency >= low_threshold {
                        let mut low_solutions = vec![
                            vec![
                                Unlockable::LilyPad,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::Tanglekelp,
                            ].into_boxed_slice(),
                        ];
                        
                        let mut high_solutions = vec![
                            vec![
                                Unlockable::LilyPad,
                                Unlockable::WallNut,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::LilyPad,
                                Unlockable::Pumpkin,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::Tanglekelp,
                                Unlockable::Squash,
                            ].into_boxed_slice(),
                        ];
                        
                        if max_frequency < high_threshold {
                            low_solutions.append(&mut high_solutions);
                        }
                        
                        if !self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions, 1) {
                            let threshold = if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions, 1) {
                                high_threshold
                            } else {
                                low_threshold
                            };
                            
                            threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                        }
                    }
                }
            }
            
            for (zombie_type, true_max) in [ //I will add zombies here as needed
                (ZombieType::Dolphinrider,7.0),
                (ZombieType::SubmarineZombie,2.0), //I don't like these guys
                (ZombieType::BungiZombie,2.0), //its really funny when you get a bungee spam level, but it can be very problematic too
            ] {
                let zombie_idx = *zombie_map.get(&zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type:?}\""));
                threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(true_max);
            }
            
            if level.level_type == LevelType::Roof && level.flags.unwrap_or(1) == 1 {
                for (zombie_type, true_max) in [ //I will add zombies here as needed
                    (ZombieType::PogoZombie,2.0),
                    (ZombieType::SuperPogoZombie,0.8),
                    (ZombieType::JackboxJumpZombie,0.8),
                ] {
                    let zombie_idx = *zombie_map.get(&zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type:?}\""));
                    threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(true_max);
                }
            }
            
            let mut removed_zombies: FxHashSet<u32> = HashSet::with_capacity_and_hasher(16, BuildHasherDefault::default());
            let mut new_spawns: Vec<(u32, u32)> = Vec::with_capacity(16);
            let one_eigth_curved = 10f64.powf(Self::weight_curve(0x1FFF_FFFF));
            for (i, (threshold, zombie)) in threshold_table.iter().zip(zombie_data.iter()).enumerate() {
                if let Some(weight) = spawns_map.get(&(i as u32)) {
                    let (max_freq, _) = *spawn_data.max_frequency.get(&(i as u32)).unwrap();
                    let first_flag_cnt = *spawn_data.first_flag_totals.get(&(i as u32)).unwrap();
                    let new_val = (*threshold / max_freq.max(first_flag_cnt)) as f64 * *weight as f64 / zombie.default_weight as f64;
                    if new_val > one_eigth_curved {
                        let new_weight = (new_val * zombie.default_weight as f64).round() as u32;
                        new_spawns.push((i as u32, new_weight.min(*weight)));
                    } else {
                        removed_zombies.insert(i as u32);
                    }
                }
            }
            
            let new_frequencies = self.compute_zombie_freq_data_cached(&new_spawns, level_idx as usize).unwrap();
            let mut bad_zombie_weight = 1.0;
            let mut bad_zombies: FxHashMap<u32,u32> = HashMap::with_capacity_and_hasher(16, BuildHasherDefault::default());
            
            for (i, weight) in new_spawns {
                let threshold      = threshold_table[i as usize];
                let zombie         = &zombie_data[i as usize];
                let (max_freq, _)  = *new_frequencies.max_frequency.get(&i).unwrap();
                let first_flag_cnt = *new_frequencies.first_flag_totals.get(&i).unwrap();
                let new_val        = (threshold / max_freq.max(first_flag_cnt)) as f64 * weight as f64 / zombie.default_weight as f64;
                let new_weight     = (new_val * zombie.default_weight as f64).round() as u32;
                let old_weight     = *spawns_map.get(&i).unwrap();
                
                if new_val <= one_eigth_curved {
                    removed_zombies.insert(i);
                } else if new_weight < old_weight {
                    let old_val = Self::weight_curve_inverse((old_weight as f64 / zombie.default_weight as f64).log10());
                    let new_val = Self::weight_curve_inverse(new_val.log10());
                    bad_zombie_weight /= 1f64 + new_val - old_val;
                    bad_zombies.insert(i, new_weight);
                }
            }
            
            bad_zombie_weight /= 8f64.powi(removed_zombies.len() as i32);
            
            for zombie_idx in removed_zombies {
                bad_zombies.insert(zombie_idx, 0);
            }
            
            ret.push(ImpossibleReason::HardZombies(bad_zombie_weight.sqrt(), bad_zombies)); //sqrt has specified precision, not that the rest of the code cares
        }
        
        
        
        let unlockable_useful_min_map: HashMap<Unlockable,(u8,u8,u8)> = [
            (Unlockable::CherryBomb,  (0x20,0x98,0xD5)),
            (Unlockable::PotatoMine,  (0x00,0x98,0xCD)),
            (Unlockable::Chomper,     (0x00,0xC8,0xD5)),
            (Unlockable::SmallPuff,   (0x50,0xE0,0xFF)),
            (Unlockable::HypnoShroom, (0x00,0x98,0xD5)),
            (Unlockable::IceShroom,   (0x20,0x98,0x70)),
            (Unlockable::DoomShroom,  (0x10,0x80,0xD5)),
            (Unlockable::LilyPad,     (0x00,0xC0,0xC0)),
            (Unlockable::Squash,      (0x40,0xB0,0xC0)),
            (Unlockable::ThreePeater, (0x60,0xF0,0x97)),
            (Unlockable::Tanglekelp,  (0x40,0xB0,0xFF)),
            (Unlockable::TorchWood,   (0x00,0xE0,0xA0)),
            (Unlockable::Jalapeno,    (0x20,0x98,0xD5)),
            (Unlockable::SeaShroom,   (0x50,0xE0,0xFF)),
            (Unlockable::Pot,         (0x00,0xC0,0xC0)),
            (Unlockable::Melonpult,   (0x40,0xF0,0xA0)),
            (Unlockable::Present,     (0x00,0x80,0xC0)),
            (Unlockable::EndoFlame,   (0x00,0xC0,0xFF)),
        ].into_iter().collect();
        let mut unlockable_importance = [0u32; 41];
        for (solutions, importance) in used_solutions {
            let mut total_weight = 0f64;
            let mut solutions_weight_vec: Vec<(Box<[Unlockable]>, f64)> = Vec::with_capacity(solutions.len());
            for solution in solutions {
                let mut weight = 1f64;
                for unlockable in &solution {
                    weight += unlockable_importance[*unlockable as usize] as f64;
                }
                solutions_weight_vec.push((solution, total_weight));
                total_weight += weight;
            }
            assert_ne!(solutions_weight_vec.len(), 0);
            assert_ne!(total_weight, 0f64);
            let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
            let idx = solutions_weight_vec.partition_point(|(_, csum)| *csum <= val);
            let solution = &solutions_weight_vec[idx - 1].0;
            for unlockable in solution {
                unlockable_importance[*unlockable as usize] += importance;
            }
        }
        let restrictions_data = self.restrictions_data.as_ref().unwrap();
        let plant_data = restrictions_data.level_plants.get(&(level_idx as u8)).unwrap();
        let mut weight_div = 1f64;
        let mut bad_plants_vec: Vec<(Unlockable,u8,u8,u8)> = Vec::with_capacity(16);
        for (unlockable_id, importance) in unlockable_importance.into_iter().enumerate() {
            if importance > 0 {
                let unlockable: Unlockable = unsafe { transmute(unlockable_id as i8) };
                let plant_true_idx = *restrictions_data.plant_map.get(&format!("{unlockable:?}")).unwrap();
                let (min_useful_fr, max_useful_cd, max_useful_cs) = unlockable_useful_min_map.get(&unlockable).unwrap_or(&(0x50,0xE0,0xE0));
                let maximum_firerate = if *min_useful_fr != 0x00 {
                    ((255f32 - *min_useful_fr as f32) / (0.8 + 0.2 * importance as f32)).round() as u8
                } else {
                    0xFF
                };
                let maximum_cooldown = if *max_useful_cd != 0xFF {
                    (*max_useful_cd as f32 / (0.8 + 0.2 * importance as f32)).round() as u8
                } else {
                    0xFF
                };
                let maximum_cost = if *max_useful_cs != 0xFF {
                    (*max_useful_cs as f32 / (0.8 + 0.2 * importance as f32)).round() as u8
                } else {
                    0xFF
                };
                //println!("{unlockable:?}: ({:.3},{:.3},{:.3})", 1./display_mul(maximum_firerate), display_mul(maximum_cooldown), display_mul(maximum_cost));
                let current_cooldown = plant_data.menu[unlockable_id].0;
                let current_cost     = plant_data.menu[unlockable_id].1;
                let current_firerate = plant_data.all[plant_true_idx as usize];
                
                if  current_cooldown > maximum_cooldown ||
                    current_cost > maximum_cost ||
                    current_firerate > maximum_firerate {
                    
                    bad_plants_vec.push((unlockable,maximum_firerate,maximum_cost,maximum_cooldown));
                    weight_div += ((current_firerate as f64 - maximum_firerate as f64) / 255.).max(0.);
                    weight_div += ((current_cost     as f64 - maximum_cost     as f64) / 255.).max(0.);
                    weight_div += ((current_cooldown as f64 - maximum_cooldown as f64) / 255.).max(0.);
                }
            }
        }
        
        if !bad_plants_vec.is_empty() {
            ret.push(ImpossibleReason::BadPlants(1./weight_div.sqrt(), bad_plants_vec));
        }
        
        if ret.is_empty() {
            Ok(())
        } else {
            Err(ret)
        }
    }
    
    fn upgrade_to_plant(upgrade: Unlockable) -> Unlockable {
        match upgrade {
            Unlockable::TallNut => Unlockable::WallNut,
            Unlockable::SpikeRock => Unlockable::Caltrop,
            Unlockable::CattailPlant => Unlockable::LilyPad,
            Unlockable::GloomShroom => Unlockable::FumeShroom,
            Unlockable::CobCannon => Unlockable::Cornpult,
            other => other,
        }
    }
    
    fn pick_level(
        &mut self,
        remaining_levels: &Vec<u8>,
        predetermined_level_plants: &FxHashMap<u8, (Unlockable, f32)>,
        blacklist_set: &FxHashSet<u32>,
        cattail_girl: bool,
        rng: &mut ChaCha8Rng,
        seed: u64,
    ) -> usize {
        let mut possible_levels: SmallVec<[(usize,f64); 64]> = SmallVec::new();
        let mut impossible_levels: SmallVec<[usize; 64]> = SmallVec::new();
        let mut total_weight = 0f64;
        
        for level_idx in remaining_levels {
            let mut level_weight = 1f64;
            
            if !(blacklist_set.contains(&(*level_idx as u32)) && remaining_levels.len() > 15) &&
                match self.is_level_possible(*level_idx as u32, if cattail_girl {45 - remaining_levels.len() as u32} else {0}, seed) {
                Ok(()) => true,
                Err(reasons) => {
                    let mut possible = true;
                    for reason in reasons {
                        match reason {
                            ImpossibleReason::NoWaterSolution |
                            ImpossibleReason::InsufficientWaterSolution |
                            ImpossibleReason::NoPot |
                            ImpossibleReason::FourFlag => possible = false,
                            ImpossibleReason::HardZombies(weight_mul, zombie_modifications) => {
                                level_weight *= weight_mul;
                                let restrictions_data = self.restrictions_data.as_mut().unwrap();
                                let mut zombies = restrictions_data.level_spawns.get(level_idx).unwrap().clone();
                                let mut remove_idxs: SmallVec<[usize; 16]> = SmallVec::new();
                                for (i, (zombie, weight)) in zombies.iter_mut().enumerate() {
                                    if let Some(new_weight) = zombie_modifications.get(zombie) {
                                        *weight = *new_weight;
                                        if *new_weight == 0 {
                                            remove_idxs.push(i);
                                        }
                                    }
                                }
                                remove_idxs.sort_unstable();
                                for i in remove_idxs.iter().rev() {
                                    zombies.remove(*i);
                                }
                                restrictions_data.modified_level_spawns.insert(*level_idx, zombies);
                            }
                            ImpossibleReason::BadPlants(weight_mul, new_plants) => {
                                level_weight *= weight_mul;
                                let restrictions_data = self.restrictions_data.as_mut().unwrap();
                                let mut plants = (*restrictions_data.level_plants.get(level_idx).unwrap()).clone();
                                for (unlockable, max_firerate, max_cost, max_cooldown) in new_plants {
                                    let plant_true_idx = *restrictions_data.plant_map.get(&format!("{unlockable:?}")).unwrap();
                                    let (cd, cs) = &mut plants.menu[unlockable as usize];
                                    let fr = &mut plants.all[plant_true_idx as usize];
                                    
                                    *cd = (*cd).min(max_cooldown);
                                    *cs = (*cs).min(max_cost);
                                    *fr = (*fr).min(max_firerate);
                                }
                                restrictions_data.modified_level_plants.insert(*level_idx, plants);
                            }
                        }
                    }
                    possible
                },
            } {
                if let Some((_, new_weight)) = predetermined_level_plants.get(level_idx) {
                    level_weight = *new_weight as f64;
                }
                possible_levels.push((*level_idx as usize, total_weight));
                total_weight += level_weight;
            } else {
                impossible_levels.push(*level_idx as usize);
            }
        }
        
        if possible_levels.is_empty() {
            panic!("These levels were impossible to satisfy: {impossible_levels:?}");
        }
        assert_ne!(possible_levels.len(), 0);
        assert_ne!(total_weight, 0f64);
        
        let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
        let idx = possible_levels.partition_point(|(_, csum)| *csum <= val);
        //println!("{val}, {possible_levels:?}");
        possible_levels[idx - 1].0
    }
    
    fn assign_solutions(
        &mut self,
        possible_solutions: FxHashMap<Problem, Solutions>,
        predetermined_level_plants: &mut FxHashMap<u8, (Unlockable, f32)>,
        remaining_levels: &mut Vec<u8>,
        blacklist_set: &FxHashSet<u32>,
        rng: &mut ChaCha8Rng,
        seed: u64,
    ) {
        let mut idx_vec: Vec<(u32, u32)> = Vec::with_capacity(possible_solutions.len());
        for i in 0..possible_solutions.len() {
            idx_vec.push((i as u32, rng.next_u32()));
        }
        idx_vec.sort_by_key(|(_, key)| *key);
        let problem_solution_vec: Vec<(Problem, Solutions)> = possible_solutions.into_iter().collect();
        let problem_solution_vec: Vec<(Problem, Solutions)> = idx_vec.iter().map(|(idx, _)| problem_solution_vec[*idx as usize].clone()).collect();
        
        for (problem, possible_solutions) in problem_solution_vec.iter() {
            let restrictions_data = self.restrictions_data.as_ref().unwrap();
            let mut solution_weights: Vec<f64> = Vec::with_capacity(possible_solutions.len());
            let mut total_weight = 0f64;
            for solution in possible_solutions {
                let mut weight = 1f64;
                for unlock in solution {
                    if restrictions_data.unlocked_plants.contains(unlock) {
                        weight += 4f64;
                    }
                }
                solution_weights.push(total_weight);
                total_weight += weight;
            }
            assert_ne!(solution_weights.len(), 0);
            assert_ne!(total_weight, 0f64);
            
            let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
            let idx = solution_weights.partition_point(|csum| *csum <= val);
            let solution = &possible_solutions[idx - 1];
            
            for unlock in solution {
                let restrictions_data = self.restrictions_data.as_ref().unwrap();
                let weight = match problem {
                    Problem::Water1           => 3.,
                    Problem::Water2           => 4.,
                    Problem::Water34          => 2.,
                    Problem::Roof             => 6.,
                    Problem::Snorkle          => 1.5,
                    Problem::DarkMicheal      => 1.5,
                    Problem::HighHealth       => 5.,
                    Problem::ReallyHighHealth => 3.,
                    Problem::Gargantaur       => 1.5,
                    Problem::Balloon          => 1.5,
                    Problem::BalloonWater     => 1.5,
                    Problem::Kirov            => 1.5,
                    Problem::NoPuff           => 2.5,
                };
                if !restrictions_data.unlocked_plants.contains(unlock) {
                    let level_idx = Self::pick_level(self, remaining_levels, predetermined_level_plants, blacklist_set, false, rng, seed);
                    let level_idx_idx = remaining_levels.binary_search(&(level_idx as u8)).unwrap();
                    remaining_levels.remove(level_idx_idx);
                    let restrictions_data = self.restrictions_data.as_mut().unwrap();
                    restrictions_data.unlocked_plants.insert(*unlock);
                    predetermined_level_plants.insert(level_idx as u8, (*unlock, weight + 1.));
                } else if let Some((_, level_weight)) = predetermined_level_plants.values_mut().find(|(level_unlock, _)| level_unlock == unlock) {
                    *level_weight += weight;
                }
            }
        }
    }
    
    pub fn restrictions(seed: u64, meta: &IL2CppDumper, fuse_data: &FxHashMap<u32,[u32;2]>) -> Self {
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        let level_data = LEVEL_DATA.get().unwrap();
        
        let mut level_rng   = ChaCha8Rng::seed_from_u64(seed ^ hash_str("level_rng"));
        let mut weights_rng = ChaCha8Rng::seed_from_u64(seed ^ hash_str("zombie_weights"));
        let mut plants_rng  = ChaCha8Rng::seed_from_u64(seed ^ hash_str("plant_order"));
        
        let sound_seeds = Self::randomise_sounds(seed ^ hash_str("Sounds"));
        
        let mut ret = Self {
            level_order: Vec::with_capacity(45),
            plant_order: vec![0xFF; 48],
            weights: Some(Vec::new()),
            firerates: Some(Vec::new()),
            cooldowns: Some(Vec::new()),
            costs: Some(Vec::new()),
            spawns: Some(Vec::new()),
            freqs: Some(Vec::new()),
            sound_seeds: Some(sound_seeds),
            restrictions_data: Some(RestrictionsData {
                frequency_cache: HashMap::default(),
                level_spawns: HashMap::default(),
                modified_level_spawns: HashMap::default(),
                level_plants: HashMap::default(),
                modified_level_plants: HashMap::default(),
                plant_map: HashMap::default(),
                unlocked_plants: HashSet::default(),
            }),
        };
        
        ret.randomise_plant_attrs(meta, fuse_data, seed);
        
        let mut remaining_levels: Vec<u8> = (2..=45).collect();
        ret.level_order.push(1);
        if let Some(weights) = ret.weights.as_mut() {
            weights.push(vec![0xA0, 0xF, 0, 0]);
        }
        if let Some(freqs) = ret.freqs.as_mut() {
            freqs.push(104f32.to_ne_bytes().to_vec());
        }
        if let Some(spawns) = ret.spawns.as_mut() {
            spawns.push(vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        }
        if let Some(firerates) = ret.firerates.as_mut() {
            firerates.push(Vec::new());
        }
        if let Some(cooldowns) = ret.cooldowns.as_mut() {
            cooldowns.push(Vec::new());
        }
        if let Some(costs) = ret.costs.as_mut() {
            costs.push(Vec::new());
        }
        
        let restrictions_data = ret.restrictions_data.as_mut().unwrap();
        
        let mut blacklist_vec: Vec<(u32, u32)> = Vec::with_capacity(32);
        let mut blacklist_set: FxHashSet<u32> = HashSet::with_capacity_and_hasher(15, BuildHasherDefault::default());
        for (i, level) in level_data.iter().enumerate().skip(1) {
            if let Some(flags) = level.flags {
                if level.conveyor_plants.is_none() && flags > 1 {
                    blacklist_vec.push((i as u32 + 1, level_rng.next_u32()))
                }
            }
        }
        blacklist_vec.sort_by_key(|(_, key)| *key);
        for (level, _) in blacklist_vec.iter().take(15) {
            blacklist_set.insert(*level);
        }
        for i in 2..=45 {
            let mut still_blacklist = false;
            let mut vec = Vec::new();
            let mut bitfield = Self::randomise_spawns_no_restrictions(
                seed ^ hash_str(&i.to_string()),
                i,
                if blacklist_set.contains(&(i as u32)) {31} else {1},
            );
            for (byte_idx, byte) in bitfield.iter_mut().enumerate() {
                loop {
                    let bit_pos = byte.trailing_zeros();
                    if bit_pos == 8 {
                        break;
                    }
                    *byte ^= 1 << bit_pos;
                    let idx = bit_pos as usize + byte_idx * 8;
                    let mut weight_mul = 10f64.powf(Self::weight_curve(weights_rng.next_u32()));
                    if idx == 0 {
                        weight_mul = weight_mul.max(1.0);
                    }
                    vec.push((idx as u32, (weight_mul * zombie_data[idx].default_weight as f64).round() as u32));
                    if zombie_data[idx].flags.contains(ZombieFlags::IS_ODYSSEY) {
                        still_blacklist = true;
                    }
                }
            }
            restrictions_data.level_spawns.insert(i as u8, vec);
            if !still_blacklist {
                blacklist_set.remove(&(i as u32));
            }
        }
        
        let mut plant_order: Vec<Unlockable> = Vec::with_capacity(41);
        let first_plant_options = [
            Unlockable::CherryBomb,
            Unlockable::Chomper,
            Unlockable::SmallPuff,
            Unlockable::FumeShroom,
            Unlockable::IceShroom,
            Unlockable::Squash,
            Unlockable::ThreePeater,
            Unlockable::TorchWood,
            Unlockable::StarFruit,
            Unlockable::Pumpkin,
            Unlockable::Magnetshroom,
            Unlockable::Melonpult,
        ];
        
        let first_plant = first_plant_options[((plants_rng.next_u32() as u64 * first_plant_options.len() as u64) >> 32) as usize];
        plant_order.push(first_plant);
        restrictions_data.unlocked_plants.insert(Unlockable::Peashooter);
        restrictions_data.unlocked_plants.insert(Unlockable::SunFlower);
        restrictions_data.unlocked_plants.insert(first_plant);
        let mut predetermined_level_plants: FxHashMap<u8, (Unlockable, f32)> = HashMap::default();
        predetermined_level_plants.insert(1, (first_plant, 999.0));
        
        let possible_solutions_part_1 = Self::get_solutions_1();
        let possible_solutions_part_2 = Self::get_solutions_2();
        let possible_solutions_part_3 = Self::get_solutions_3();
        
        ret.assign_solutions(possible_solutions_part_1, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng, seed);
        ret.assign_solutions(possible_solutions_part_2, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng, seed);
        ret.assign_solutions(possible_solutions_part_3, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng, seed);
        
        println!("Chosen plant solutions: {:?}", predetermined_level_plants.values().collect::<Vec<_>>());
        println!("Blacklist: {blacklist_set:?}");
        
        let mut remaining_plants: Vec<Unlockable> = vec![
            Unlockable::CherryBomb,
            Unlockable::WallNut,
            Unlockable::PotatoMine,
            Unlockable::Chomper,
            Unlockable::SmallPuff,
            Unlockable::FumeShroom,
            Unlockable::HypnoShroom,
            Unlockable::ScaredyShroom,
            Unlockable::IceShroom,
            Unlockable::DoomShroom,
            Unlockable::LilyPad,
            Unlockable::Squash,
            Unlockable::ThreePeater,
            Unlockable::Tanglekelp,
            Unlockable::Jalapeno,
            Unlockable::Caltrop,
            Unlockable::TorchWood,
            Unlockable::SeaShroom,
            Unlockable::Plantern,
            Unlockable::Cactus,
            Unlockable::Blover,
            Unlockable::StarFruit,
            Unlockable::Pumpkin,
            Unlockable::Magnetshroom,
            Unlockable::Cabbagepult,
            Unlockable::Pot,
            Unlockable::Cornpult,
            Unlockable::Garlic,
            Unlockable::Umbrellaleaf,
            Unlockable::Marigold,
            Unlockable::Melonpult,
            Unlockable::PresentZombie,
            Unlockable::EndoFlame,
            Unlockable::Present,
            Unlockable::TallNut,
            Unlockable::SpikeRock,
            Unlockable::CattailPlant,
            Unlockable::GloomShroom,
            Unlockable::CobCannon,
        ]; //cringe
        
        let mut to_remove = Vec::with_capacity(32);
        let mut forced_plants = FxHashMap::default();
        for (level, (plant, _)) in &predetermined_level_plants {
            forced_plants.insert(*plant, *level);
        }
        for (i, unlockable) in remaining_plants.iter().enumerate() {
            if forced_plants.contains_key(unlockable) {
                to_remove.push(i);
            }
        }
        for i in to_remove.into_iter().rev() {
            remaining_plants.remove(i);
        }
        
        let mut remaining_levels: Vec<u8> = (2..=45).collect();
        let restrictions_data = ret.restrictions_data.as_mut().unwrap();
        restrictions_data.unlocked_plants.drain();
        restrictions_data.unlocked_plants.insert(first_plant);
        restrictions_data.unlocked_plants.insert(Unlockable::Peashooter);
        restrictions_data.unlocked_plants.insert(Unlockable::SunFlower);
        
        while !remaining_levels.is_empty() {
            let level_idx = Self::pick_level(&mut ret, &remaining_levels, &predetermined_level_plants, &blacklist_set, true, &mut level_rng, seed);
            let level_idx_idx = remaining_levels.binary_search(&(level_idx as u8)).unwrap();
            let restrictions_data = ret.restrictions_data.as_mut().unwrap();
            
            if let Some(spawns) = restrictions_data.level_spawns.remove(&(level_idx as u8)) {
                let actual_spawns = if let Some(spawns) = restrictions_data.modified_level_spawns.remove(&(level_idx as u8)) {
                    spawns
                } else {
                    spawns
                };
                let mut spawns_bitfield = vec![0; 16];
                let mut weights_vec = vec![0; zombie_data.len() * 4];
                for (idx, weight) in actual_spawns.iter() {
                    for (i, byte) in weight.to_le_bytes().iter().enumerate() {
                        weights_vec[(*idx as usize) * 4 + i] = *byte;
                    }
                    Self::xor_bit_in_bitfield(*idx as usize, &mut spawns_bitfield);
                }
                if let Some(weights) = ret.weights.as_mut() {
                    weights.push(weights_vec);
                }
                if let Some(spawns) = ret.spawns.as_mut() {
                    spawns.push(spawns_bitfield);
                }
                if ret.freqs.is_some() {
                    let data = ret.compute_zombie_freq_data_cached(&actual_spawns, level_idx).unwrap();
                    unsafe { ret.freqs.as_mut().unwrap_unchecked() }.push(data.totals);
                }
            } else {
                unreachable!();
            }
            
            let restrictions_data = ret.restrictions_data.as_mut().unwrap();
            if let Some(plants) = restrictions_data.level_plants.remove(&(level_idx as u8)) {
                let actual_plants = if let Some(plants) = restrictions_data.modified_level_plants.remove(&(level_idx as u8)) {
                    plants
                } else {
                    plants
                };
                let cd_vec: Vec<u8> = actual_plants.menu.iter().map(|(cd, _)| *cd).collect();
                let cs_vec: Vec<u8> = actual_plants.menu.iter().map(|(_, cs)| *cs).collect();
                if let Some(firerates) = ret.firerates.as_mut() {
                    firerates.push(actual_plants.all);
                }
                if let Some(cooldowns) = ret.cooldowns.as_mut() {
                    cooldowns.push(cd_vec);
                }
                if let Some(costs) = ret.costs.as_mut() {
                    costs.push(cs_vec);
                }
            } else {
                unreachable!();
            }
            
            remaining_levels.remove(level_idx_idx);
            ret.level_order.push(level_idx as u8);
            
            if let Some((plant, _)) = predetermined_level_plants.get(&(level_idx as u8)) {
                let plant = *plant; //avoid immutable borrow
                let new_plant = Self::upgrade_to_plant(plant);
                if restrictions_data.unlocked_plants.contains(&new_plant) {
                    plant_order.push(plant);
                    restrictions_data.unlocked_plants.insert(plant);
                } else {
                    plant_order.push(new_plant);
                    restrictions_data.unlocked_plants.insert(new_plant);
                    if let Some(level) = forced_plants.get(&new_plant) {
                        let (plant_2, _) = predetermined_level_plants.get_mut(level).unwrap();
                        *plant_2 = plant;
                    }
                }
            } else if !remaining_plants.is_empty() {
                let mut plant_choices = Vec::with_capacity(41);
                for (idx, plant) in remaining_plants.iter().enumerate() {
                    let new_plant = Self::upgrade_to_plant(*plant);
                    if new_plant == *plant || restrictions_data.unlocked_plants.contains(&new_plant) {
                        plant_choices.push((idx, *plant));
                    }
                }
                let idx = ((plants_rng.next_u32() as u64 * plant_choices.len() as u64) >> 32) as usize;
                let (rm_idx, plant) = plant_choices[idx];
                plant_order.push(plant);
                restrictions_data.unlocked_plants.insert(plant);
                remaining_plants.remove(rm_idx);
            }
        }
        
        ret.plant_order[0] = 0;
        ret.plant_order[1] = 0;
        
        for (plant, i) in plant_order.iter().zip(2..) {
            ret.plant_order[*plant as usize] = i;
        }
        
        println!("Plant order: {plant_order:?}");
        
        ret
    }
}

fn mul_from_u8(mul: u8) -> f32 {
    ((mul & 0x7F) as f32 / 127. + 1.) * if mul < 0x80 {0.5} else {1.}
}
