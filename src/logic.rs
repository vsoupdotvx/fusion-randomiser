use std::{collections::{HashMap, HashSet}, hash::BuildHasherDefault};

use fxhash::{FxHashMap, FxHashSet};
use rand::RngCore;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use smallvec::SmallVec;
use crate::{data::{LevelData, LevelType, Unlockable, ZombieLanes, LEVEL_DATA}, fast_map::FastMap, il2cppdump::IL2CppDumper, util::hash_str};
use crate::data::ZOMBIE_DATA;

pub struct RandomisationData {
    pub level_order:   Vec<u8>,
    pub plant_order:   Vec<u8>,
    pub weights:       Option<Vec<Vec<u8>>>,
    pub firerates:     Option<Vec<Vec<u8>>>,
    pub cooldowns:     Option<Vec<Vec<u8>>>,
    pub costs:         Option<Vec<Vec<u8>>>,
    pub spawns:        Option<Vec<Vec<u8>>>,
    restrictions_data: Option<RestrictionsData>,
}

struct RestrictionsData {
    frequency_cache: FxHashMap<FrequencyCacheKey, FrequencyData>,
    level_spawns: FxHashMap<u8, Vec<(u32,u32)>>,
    modified_level_spawns: FxHashMap<u8, Vec<(u32,u32)>>,
    _level_plants: FxHashMap<u8, Vec<(u8, u8, u8)>>,
    unlocked_plants: FxHashSet<Unlockable>,
}

#[allow(dead_code)]
enum ImpossibleReason {
    NoWaterSolution,
    InsufficientWaterSolution,
    NoPot,
    FourFlag,
    HardZombies(f64, FxHashMap<u32,u32>),
    BadPlants(Vec<(u8,u8)>),
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
struct FrequencyData {
    raw_averages: Vec<f32>,
    max_frequency: FxHashMap<u32, (f32, u32)>,
    first_flag_totals: FxHashMap<u32, f32>,
    first_wave_occurence_avgs: FxHashMap<u32, u32>,
}

#[derive(Hash, Clone, Eq, PartialEq)]
struct FrequencyCacheKey {
    spawns: Box<[(u32,u32)]>,
    level: usize,
}

type Solutions = Box<[Box<[Unlockable]>]>;

impl RandomisationData {
    pub fn no_restrictions(seed: u64, meta: &IL2CppDumper, fuse_data: &HashMap<u32,[u32;2]>) -> Self {
        let plant_ids     = Self::get_plant_ids(meta);
        let level_order   = Self::randomise_level_order_no_restrictions(seed);
        let plant_order   = Self::randomise_plant_order_no_restrictions(seed);
        let mut weights   = Vec::new();
        let mut firerates = Vec::new();
        let mut cooldowns = Vec::new();
        let mut costs     = Vec::new();
        let mut spawns    = Vec::new();
        
        weights.push(vec![1, 0, 0, 0]);
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
            Self::compute_zombie_freq_data_bytes(&spawns[level_true_idx - 1], &weights[level_true_idx - 1], *level_idx as usize).unwrap();
        }
        
        Self {
            level_order,
            plant_order,
            weights:   Some(weights),
            firerates: Some(firerates),
            cooldowns: Some(cooldowns),
            costs:     Some(costs),
            spawns:    Some(spawns),
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
        10.0f64.powf(((64. / 15. * num - 32. / 5.) * num + 62. / 15.) * num - 1.)
    }
    
    fn xor_bit_in_bitfield(bit: usize, bitfield: &mut [u8]) {
        bitfield[bit >> 3] ^= 1 << (bit & 7) as u8;
    }
    
    fn randomise_weights_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("zombie_weights")));
        #[allow(static_mut_refs)]
        let mut ret = Vec::with_capacity(unsafe {ZOMBIE_DATA.as_ref()}.unwrap().len() * 4);
        
        #[allow(static_mut_refs)]
        for zombie in unsafe {ZOMBIE_DATA.as_ref()}.unwrap() {
            let weight_mul = Self::weight_curve(rng.next_u32());
            for byte in ((weight_mul * zombie.default_weight as f64).round() as i32).to_le_bytes() {
                ret.push(byte);
            }
        }
        
        ret
    }
    
    fn randomise_firerates_no_restrictions(seed: u64, plant_ids: &[u32], fuse_data: &HashMap<u32,[u32;2]>) -> Vec<u8> {
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
        #[allow(static_mut_refs)]
        let level = &unsafe{LEVEL_DATA.as_ref()}.unwrap()[level_idx - 1];
        
        for zombie_type in &level.default_zombie_types {
            Self::xor_bit_in_bitfield(*zombie_type as usize, &mut ret);
        }
        
        #[allow(static_mut_refs)]
        for (i, zombie) in unsafe {ZOMBIE_DATA.as_ref()}.unwrap().iter().enumerate() {
            if let ZombieLanes::Water = zombie.allowed_lanes {
                match level.level_type {
                    LevelType::Pool |
                    LevelType::Fog => {}
                    _ => continue
                }
            }
            if zombie.is_odyssey && level_true_idx <= 30 {
                continue;
            }
            
            let val = rng.next_u32();
            if (ret[i >> 8] & 1 << (i & 7) as u8) == 0 {
                if (!zombie.is_odyssey && val >= u32::MAX / 20) ||
                    (zombie.is_odyssey && val >= u32::MAX / 60) {
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
    
    fn set_fusion_firerates(firerates: &mut [u8], plant_ids: &[u32], fuse_data: &HashMap<u32,[u32;2]>) {
        let mut plant_lookup: HashMap<u32,u32> = HashMap::with_capacity(plant_ids.len());
        
        for (i, plant_id) in plant_ids.iter().enumerate() {
            plant_lookup.insert(*plant_id, i as u32);
        }
        
        fn get_fused_firerate(
            fuse_plants: [u32;2],
            plant_lookup: &HashMap<u32,u32>,
            fuse_data: &HashMap<u32,[u32; 2]>,
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
    
    
    
    
    
    
    
    
    fn weight_curve_inverse(float: f64) -> f64 {
        let x   = float.log10();
        let x2  = x*x;
        ((0.43080993081*x2-0.835882783883)*x2+0.905073260073)*x+0.5 //a crude approximation of the inverse of the input polynomial
    }
    
    fn get_zombie_map() -> FxHashMap<&'static str, u32> {
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        
        let mut ret = HashMap::with_capacity_and_hasher(128, BuildHasherDefault::default());
        
        for (i, zombie) in zombie_data.iter().enumerate() {
            ret.insert(zombie.id_name, i as u32);
        }
        
        ret
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
                    Unlockable::Blower,
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
                    Unlockable::Blower,
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
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        
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
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        #[allow(static_mut_refs)]
        let level_data = unsafe {LEVEL_DATA.as_ref()}.unwrap();
        
        let mut spawn_vec_pre_10 = spawn_vec.to_vec();
        let mut pre_10_map = Vec::from_iter(0..spawn_vec.len());
        
        for (i, (idx, _, _)) in spawn_vec.iter().enumerate().rev() {
            if zombie_data[*idx as usize].is_elite {
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
            
            let mut odds_old: FastMap = FastMap::with_size(256, spawn_vec.len() as u32);
            odds_old.insert_probability(&vec![0u16; spawn_vec.len()], 1f64, wave * 5 / 3);
            let mut zombie_odds: Vec<f64> = vec![0f64; spawn_vec.len()];
            let mut loss_mul = 1f64;
            
            loop {
                let mut odds: FastMap = FastMap::with_size(odds_old.len(), spawn_vec.len() as u32);
                
                for (zombies, (chance_1, remaining_points)) in &mut odds_old {
                    for (i, chance_2) in spawns_lut[usize::min(remaining_points as usize, spawns_lut.len()) - 1].iter().enumerate() {
                        let chance = chance_1 * *chance_2;
                        let choice_wavepoints = spawn_vec[i].2 as isize;
                        let mut zombies = zombies.to_vec(); //this is somehow faster than using an array and memcpying it, i guess through stack allocation?
                        zombies[i] += 1;
                        if remaining_points > choice_wavepoints {
                            if chance > 1f64 / 16_777_216f64 {
                                odds.insert_probability(&zombies, chance, remaining_points - choice_wavepoints);
                            } else {
                                loss_mul -= chance;
                            }
                        } else {
                            for (cnt, current_odds) in zombies.iter().zip(zombie_odds.iter_mut()) {
                                *current_odds += *cnt as f64 * chance;
                            }
                        }
                    }
                }
                
                if odds.is_empty() {
                    break;
                }
                
                odds_old = odds;
            }
            
            loss_mul = 1f64 / loss_mul;
            for chance in zombie_odds.iter_mut() {
                *chance *= loss_mul;
            }
            
            zombie_odds
        }
        
        let processed_waves: Vec<isize> = vec![1, 2, 3, 5, 9, 10, 16];
        let wave_max = level_data[level - 1].flags? as isize * 10;
        let mut freq_array = vec![f32::NAN; wave_max as usize * spawn_vec.len()];
        
        for wave in processed_waves.iter().take_while(|wave| **wave <= wave_max) {
            let spawn_data = if *wave < 10 {&spawn_vec_pre_10} else {spawn_vec};
            let zombie_freq = compute_freq_for_wave(spawn_data, *wave);
            if *wave < 10 {
                let off = spawn_vec.len() * (*wave as usize - 1);
                for dst in freq_array
                    .iter_mut()
                    .skip(spawn_vec.len() * (*wave as usize - 1))
                    .take(spawn_vec.len()) {
                    *dst = 0f32;
                }
                for (src, idx) in zombie_freq.iter().zip(pre_10_map.iter()) {
                    freq_array[off + *idx] = *src as f32;
                }
            } else {
                for (dst, src) in freq_array
                    .iter_mut()
                    .skip(spawn_vec.len() * (*wave as usize - 1))
                    .take(spawn_vec.len())
                    .zip(zombie_freq.iter()) {
                    *dst = *src as f32;
                }
            }
        }
        
        for wave in 1 .. wave_max as usize {
            let dst_off = wave * spawn_vec.len();
            if freq_array[dst_off].is_nan() {
                let point = processed_waves.partition_point(|pwave| *pwave < wave as isize + 1);
                let wave1 = processed_waves[point - 1] as usize - 1;
                let src1_off = wave1 * spawn_vec.len();
                let mul1 = (wave * 5 / 3) as f32 / (wave1 * 5 / 3) as f32;
                if point == processed_waves.len() {
                    for (dst, src1) in (dst_off .. dst_off + spawn_vec.len()).zip(src1_off .. src1_off + spawn_vec.len()) { //have to use indices to prevent mutable borrow + immutable borrow
                        freq_array[dst] = freq_array[src1] * mul1;
                    }
                } else {
                    let wave2 = processed_waves[point] as usize - 1;
                    let src2_off = wave2 * spawn_vec.len();
                    let mul2 = (wave * 5 / 3) as f32 / (wave2 * 5 / 3) as f32;
                    for ((dst, src1), src2) in
                        (dst_off .. dst_off + spawn_vec.len())
                        .zip(src1_off .. src1_off + spawn_vec.len())
                        .zip(src2_off .. src2_off + spawn_vec.len()) { //have to use indices to prevent mutable borrow + immutable borrow
                        freq_array[dst] = (freq_array[src1] * mul1 + freq_array[src2] * mul2) * 0.5;
                    }
                }
            }
        }
        
        for wave in [9, 19, 29, 39].into_iter().take_while(|wave| *wave <= wave_max) {
            if wave == 9 && wave_max >= 20 {
                let src_off = spawn_vec.len() * 19;
                let dst_off = spawn_vec.len() * 9;
                for (dst, src) in (dst_off .. dst_off + spawn_vec.len()).zip(src_off .. src_off + spawn_vec.len()) { //have to use indices to prevent mutable borrow + immutable borrow
                    freq_array[dst] = freq_array[src];
                }
            } else {
                let off = spawn_vec.len() * wave as usize;
                for avg_zombies in freq_array.iter_mut().skip(off).take(spawn_vec.len()) {
                    *avg_zombies *= 2f32;
                }
            }
        }
        
        let mut max_frequency = HashMap::default();
        let mut first_flag_totals = HashMap::default();
        let mut first_wave_occurence_avgs = HashMap::default();
        
        for (i, (id, _, _)) in spawn_vec.iter().enumerate() {
            let mut max_freq      = 0f32;
            let mut max_wave      = 1;
            let mut total_freq    = 0f32;
            let mut total_freq_ff = 0f32;
            let mut first_wave    = 0;
            for (freq, j) in freq_array.iter().skip(i).step_by(spawn_vec.len()).zip(1..) {
                let freq_mul = if j % 10 == 9 {0.5} else {1.0};
                if *freq * freq_mul > max_freq {
                    max_freq = *freq;
                    max_wave = j;
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
            first_wave_occurence_avgs.insert(*id, first_wave);
        }
        
        Some(FrequencyData {
            raw_averages: freq_array,
            max_frequency,
            first_flag_totals,
            first_wave_occurence_avgs,
        })
    }
    
    fn compute_zombie_freq_data_cached(&mut self, level_spawns: &[(u32,u32)], level: usize) -> Option<FrequencyData> {
        let key = FrequencyCacheKey {
            spawns: level_spawns.into(),
            level,
        };
        
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        
        let mut spawn_vec: Vec<(u32, u32, u32)> = Vec::with_capacity(level_spawns.len());
        for (idx, weight) in level_spawns.iter() {
            spawn_vec.push((*idx, *weight, zombie_data[*idx as usize].default_points));
        }
        
        spawn_vec.sort_by_key(|(_, _, points)| *points);
        
        let frequency_cache = &mut self.restrictions_data.as_mut().unwrap().frequency_cache;
        if let Some(entry) = frequency_cache.get(&key) {
            return Some(entry.clone());
        }
        
        if let Some(freq_data) = Self::compute_zombie_freq_data(&spawn_vec, level) {
            frequency_cache.insert(key, freq_data.clone());
            Some(freq_data)
        } else {
            None
        }
    }
    
    fn is_any_solution_satisfied(&mut self, solutions: &Solutions, level: &LevelData, used_solutions: &mut FxHashMap<Solutions, u32>) -> bool { //also mutates self
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
        
        used_solutions.entry(vec.into_boxed_slice()).and_modify(|x| *x += 1).or_insert(1);
        
        solution_found
    }
    
    fn is_level_possible(&mut self, level_idx: u32, level_true_idx: u32) -> Result<(),Vec<ImpossibleReason>> {
        let mut ret = Vec::new();
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        #[allow(static_mut_refs)]
        let level = &unsafe {LEVEL_DATA.as_ref()}.unwrap()[level_idx as usize - 1];
        let solutions = Self::get_solutions_all();
        
        let mut used_solutions: FxHashMap<Solutions, u32> = HashMap::with_capacity_and_hasher(64, BuildHasherDefault::default());
        
        if level.conveyor_plants.is_some() {
            
        } else {
            let flags = level.flags.unwrap();
            match level.level_type {
                LevelType::Pool |
                LevelType::Fog => {
                    match flags {
                        1 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water1).unwrap(), level, &mut used_solutions)
                            && !(2..=4).contains(&(level_true_idx % 7)) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        2 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water2).unwrap(), level, &mut used_solutions)
                            && !(2..=4).contains(&(level_true_idx % 7)) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        3 |
                        4 => if !self.is_any_solution_satisfied(solutions.get(&Problem::Water34).unwrap(), level, &mut used_solutions) {
                            ret.push(ImpossibleReason::NoWaterSolution);
                        }
                        _ => unreachable!(),
                    }
                }
                LevelType::Roof => {
                    match flags {
                        1 => {}
                        _ => if !self.is_any_solution_satisfied(solutions.get(&Problem::Roof).unwrap(), level, &mut used_solutions) {
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
            for (zombie_type, low_threshold, high_threshold, really_high_threshold) in [ //high health
                ("ZombieType::FootballZombie",0.2,0.6,3.0),
                ("ZombieType::DollSilver",0.1,0.6,1.5),
                ("ZombieType::DriverZombie",0.2,0.6,2.0),
                ("ZombieType::SuperDriver",0.2,0.6,2.0),
                ("ZombieType::SuperJackboxZombie",0.2,0.4,3.0),
                ("ZombieType::SuperPogoZombie",0.1,0.8,1.5),
                ("ZombieType::MachineNutZombie",0.1,1.2,2.0),
                ("ZombieType::SnowZombie",0.2,1.0,4.0),
                ("ZombieType::IronPeaZombie",0.1,0.8,2.0),
                ("ZombieType::TallNutFootballZombie",0.1,0.8,1.5),
                ("ZombieType::TallIceNutZombie",0.3,0.8,1.5),
                ("ZombieType::CherryCatapultZombie",0.1,0.8,1.5),
                ("ZombieType::IronPeaDoorZombie",0.1,0.4,0.5),
                ("ZombieType::JalaSquashZombie",0.05,0.4,0.5),
                ("ZombieType::GatlingFootballZombie",0.05,0.4,0.5),
                ("ZombieType::SuperSubmarine",0.05,0.4,0.5),
                ("ZombieType::JacksonDriver",0.025,0.2,0.25),
                ("ZombieType::FootballDrown",0.1,0.8,1.5),
                ("ZombieType::JackboxJumpZombie",0.05,0.4,0.5),
                ("ZombieType::SuperMachineNutZombie",0.05,0.4,0.5),
                ("ZombieType::ObsidianImpZombie",0.1,0.8,1.5),
                ("ZombieType::DiamondRandomZombie",0.025,0.2,0.25),
                ("ZombieType::DrownpultZombie",0.1,0.8,1.5),
            ] {
                let zombie_idx = *zombie_map.get(zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type}\""));
                let zombie = &zombie_data[zombie_idx as usize];
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    if *max_frequency < low_threshold{
                        continue;
                    }
                    
                    let mut low_solutions = vec![
                        vec![
                            Unlockable::Squash,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Jalapeno,
                        ].into_boxed_slice(),
                    ];
                    
                    if !zombie.is_vehicle && zombie.can_hypno {
                        low_solutions.push(vec![
                            Unlockable::HypnoShroom,
                        ].into_boxed_slice());
                    }
                    
                    let mut high_solutions = vec![
                        vec![
                            Unlockable::Chomper,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Melonpult,
                        ].into_boxed_slice(),
                    ];
                    
                    if !zombie.is_vehicle && zombie.can_hypno {
                        high_solutions.push(vec![
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_boxed_slice());
                    } else if zombie.is_vehicle {
                        high_solutions.push(
                            vec![
                            Unlockable::Caltrop,
                        ].into_boxed_slice());
                    }
                    
                    let mut r_high_solutions = vec![
                        vec![
                            Unlockable::TorchWood,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Plantern,
                            Unlockable::StarFruit,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::CherryBomb,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::SmallPuff, //gatling puff is very good
                        ].into_boxed_slice(),
                    ];
                    
                    if zombie.is_vehicle {
                        r_high_solutions.push(vec![
                            Unlockable::Caltrop,
                            Unlockable::ThreePeater,
                        ].into_boxed_slice());
                        r_high_solutions.push(vec![
                            Unlockable::Caltrop,
                            Unlockable::SpikeRock,
                        ].into_boxed_slice());
                    }
                    
                    if zombie.is_metal {
                        r_high_solutions.push(vec![
                            Unlockable::Magnetshroom,
                        ].into_boxed_slice());
                    }
                    
                    if !self.is_any_solution_satisfied(&r_high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        let threshold = if self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            really_high_threshold
                        } else if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            high_threshold
                        } else {
                            low_threshold
                        };
                        
                        threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                    }
                }
            }
            
            for (zombie_type, low_threshold, high_threshold, really_high_threshold) in [ //very high health
                ("ZombieType::DollDiamond",0.05,0.8,1.5),
                ("ZombieType::DollGold",0.1,0.8,1.5),
                ("ZombieType::NewYearZombie",0.05,0.8,1.5),
                ("ZombieType::BlackFootball",0.05,0.4,0.75),
                ("ZombieType::UltimateFootballZombie",0.05,0.4,0.75),
                ("ZombieType::JacksonDriver",0.05,0.4,0.75),
            ] {
                let zombie_idx = *zombie_map.get(zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type}\""));
                let zombie = &zombie_data[zombie_idx as usize];
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    if *max_frequency < low_threshold{
                        continue;
                    }
                    
                    let low_solutions = if !zombie.is_vehicle && zombie.can_hypno {
                        vec![vec![
                            Unlockable::HypnoShroom,
                        ].into_boxed_slice()]
                    } else {
                        Vec::new()
                    };
                    
                    let mut high_solutions = vec![
                        vec![
                            Unlockable::Chomper,
                        ].into_boxed_slice(),
                    ];
                    
                    if !zombie.is_vehicle && zombie.can_hypno {
                        high_solutions.push(vec![
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_boxed_slice());
                    } else if zombie.is_vehicle {
                        high_solutions.push(
                            vec![
                            Unlockable::Caltrop,
                        ].into_boxed_slice());
                    }
                    
                    if zombie.is_metal {
                        high_solutions.push(vec![
                            Unlockable::Magnetshroom,
                        ].into_boxed_slice());
                    }
                    
                    let r_high_solutions = if zombie.is_metal {
                        vec![vec![
                            Unlockable::Magnetshroom,
                            Unlockable::Plantern,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Magnetshroom,
                            Unlockable::Blower,
                        ].into_boxed_slice()]
                    } else {
                        Vec::new()
                    };
                    
                    if !self.is_any_solution_satisfied(&r_high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        let threshold = if self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            really_high_threshold
                        } else if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            high_threshold
                        } else {
                            low_threshold
                        };
                        
                        threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                    }
                }
            }
            
            for (zombie_type, low_threshold, high_threshold, really_high_threshold, max_threshold, can_firepower, can_umbrella, can_cwall) in [ //evil death zombies
                ("ZombieType::DancePolZombie",0.05,0.4,0.8,2.0,true,false,false),
                ("ZombieType::JacksonZombie",0.05,0.4,0.8,2.0,true,false,false),
                ("ZombieType::ElitePaperZombie",0.05,0.4,0.8,1.5,true,false,false),
                ("ZombieType::SuperPogoZombie",0.05,0.4,0.8,2.0,true,true,false),
                ("ZombieType::MachineNutZombie",0.05,0.4,0.8,1.5,true,true,false),
                ("ZombieType::SnowGunZombie",0.05,0.4,0.8,1.5,true,false,false), //still don't know what this individual does, but they look like they belong here
                ("ZombieType::CherryShooterZombie",0.05,0.4,0.8,1.5,true,false,true),
                ("ZombieType::SuperCherryShooterZombie",0.05,0.3,0.6,1.0,false,false,true),
                ("ZombieType::CherryPaperZombie",0.05,0.4,0.8,2.0,false,false,true),
                ("ZombieType::CherryCatapultZombie",0.05,0.4,0.8,1.5,true,true,false),
                ("ZombieType::JalaSquashZombie",0.05,0.4,0.8,1.3,true,false,false),
                ("ZombieType::JacksonDriver",0.05,0.2,0.4,0.8,false,false,false),
                ("ZombieType::CherryPaperZ95",0.05,0.1,0.3,0.6,false,false,true),
                ("ZombieType::QuickJacksonZombie",0.05,0.2,0.4,0.8,true,false,false),
                ("ZombieType::JackboxJumpZombie",0.05,0.4,0.8,1.5,true,true,false),
                ("ZombieType::SuperMachineNutZombie",0.05,0.4,0.8,1.5,true,false,false),
                ("ZombieType::DolphinGatlingZombie",0.05,0.2,0.4,1.0,true,false,false),
                ("ZombieType::DrownpultZombie",0.05,0.4,0.8,2.0,false,false,true), //idk how bad these guys are actually, but they probably belong here
            ] {
                let zombie_idx = *zombie_map.get(zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type}\""));
                let zombie = &zombie_data[zombie_idx as usize];
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    if *max_frequency < low_threshold{
                        continue;
                    }
                    
                    let mut low_solutions = vec![
                        vec![
                            Unlockable::Squash,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Jalapeno,
                        ].into_boxed_slice(),
                    ];
                    
                    if !zombie.is_vehicle && zombie.can_hypno {
                        low_solutions.push(vec![
                            Unlockable::HypnoShroom,
                        ].into_boxed_slice());
                    }
                    
                    let mut high_solutions = vec![
                        vec![
                            Unlockable::Chomper,
                        ].into_boxed_slice(),
                    ];
                    
                    if !zombie.is_vehicle && zombie.can_hypno {
                        high_solutions.push(vec![
                            Unlockable::HypnoShroom,
                            Unlockable::SmallPuff,
                        ].into_boxed_slice());
                    } else if zombie.is_vehicle {
                        high_solutions.push(vec![
                            Unlockable::Caltrop,
                        ].into_boxed_slice());
                    }
                    
                    if zombie.is_metal {
                        high_solutions.push(vec![
                            Unlockable::Magnetshroom,
                        ].into_boxed_slice());
                    }
                    
                    let mut r_high_solutions = if zombie.is_metal {vec![
                        vec![
                            Unlockable::Magnetshroom,
                            Unlockable::Plantern,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Magnetshroom,
                            Unlockable::Blower,
                        ].into_boxed_slice()
                    ]} else {
                        Vec::new()
                    };
                    
                    if zombie.is_vehicle {
                        r_high_solutions.push(vec![
                            Unlockable::Caltrop,
                            Unlockable::ThreePeater,
                        ].into_boxed_slice());
                        r_high_solutions.push(vec![
                            Unlockable::Caltrop,
                            Unlockable::SpikeRock,
                        ].into_boxed_slice());
                    }
                    
                    if can_firepower {
                        for solution in if level.level_type == LevelType::Roof {vec![
                            vec![
                                Unlockable::TorchWood,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::Plantern,
                                Unlockable::StarFruit,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::CherryBomb,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::SmallPuff,
                            ].into_boxed_slice(),
                        ]} else {vec![
                            vec![
                                Unlockable::TorchWood,
                                Unlockable::Jalapeno,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::Plantern,
                                Unlockable::StarFruit,
                            ].into_boxed_slice(),
                            vec![
                                Unlockable::SmallPuff,
                            ].into_boxed_slice(),
                        ]} {
                            r_high_solutions.push(solution);
                        }
                    }
                    
                    if level.level_type != LevelType::Roof && level.flags.unwrap_or(1) != 1 {
                        if can_umbrella {
                            for solution in [
                                vec![
                                    Unlockable::Umbrellaleaf,
                                    Unlockable::Garlic,
                                ].into_boxed_slice(),
                                vec![
                                    Unlockable::Umbrellaleaf,
                                    Unlockable::Cornpult,
                                ].into_boxed_slice(),
                            ] {
                                r_high_solutions.push(solution);
                            }
                        }
                        if can_cwall {
                            r_high_solutions.push(vec![
                                Unlockable::CherryBomb,
                                Unlockable::WallNut,
                            ].into_boxed_slice());
                        }
                    }
                    
                    let threshold = if !self.is_any_solution_satisfied(&r_high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        max_threshold
                    } else if self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        really_high_threshold
                    } else if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        high_threshold
                    } else {
                        low_threshold
                    };
                    
                    threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                }
            }
            
            for (zombie_type, low_threshold, high_threshold) in [ //gargs
                ("ZombieType::Gargantuar",0.1,0.6),
                ("ZombieType::RedGargantuar",0.1,0.6),
                ("ZombieType::IronGargantuar",0.1,0.6),
                ("ZombieType::IronRedGargantuar",0.1,0.6),
                ("ZombieType::SuperGargantuar",0.05,0.3),
            ] {
                let zombie_idx = *zombie_map.get(zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type}\""));
                let zombie = &zombie_data[zombie_idx as usize];
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    if *max_frequency < low_threshold{
                        continue;
                    }
                    
                    let low_solutions = vec![
                        vec![
                            Unlockable::DoomShroom,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::CherryBomb,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Squash,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::PotatoMine,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Jalapeno,
                        ].into_boxed_slice(),
                    ];
                    
                    let mut high_solutions = vec![
                        vec![
                            Unlockable::TorchWood,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Plantern,
                            Unlockable::StarFruit,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::CherryBomb,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::SmallPuff,
                        ].into_boxed_slice(),
                    ];
                    
                    if zombie.is_metal {
                        high_solutions.push(vec![
                            Unlockable::Magnetshroom,
                        ].into_boxed_slice());
                    }
                    
                    if !self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        let threshold = if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            high_threshold
                        } else {
                            low_threshold
                        };
                        
                        threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                    }
                }
            }
            
            for (zombie_type, low_threshold, high_threshold, can_blover) in [ //balloons
                ("ZombieType::BalloonZombie",0.05,0.2,true),
                ("ZombieType::IronBallonZombie",0.05,0.2,true),
                ("ZombieType::IronBallonZombie2",0.05,0.2,true), //I think I can safely put this individual here
                ("ZombieType::KirovZombie",0.05,0.2,false),
                ("ZombieType::UltimateKirovZombie",0.05,0.2,false),
            ] {
                let zombie_idx = *zombie_map.get(zombie_type).unwrap_or_else(|| panic!("Zombie type does not exist: \"{zombie_type}\""));
                let _zombie = &zombie_data[zombie_idx as usize];
                if let Some((max_frequency, _)) = spawn_data.max_frequency.get(&zombie_idx) {
                    if *max_frequency < low_threshold{
                        continue;
                    }
                    
                    let low_solutions = vec![
                        vec![
                            Unlockable::DoomShroom,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::CherryBomb,
                        ].into_boxed_slice(),
                        vec![
                            Unlockable::Jalapeno,
                        ].into_boxed_slice(),
                    ];
                    
                    let mut high_solutions = vec![ match level.level_type {
                        LevelType::Pool |
                        LevelType::Fog => vec![
                            Unlockable::Cactus,
                            Unlockable::Plantern,
                            Unlockable::LilyPad,
                        ].into_boxed_slice(),
                        _ => vec![
                            Unlockable::Cactus,
                            Unlockable::Plantern,
                        ].into_boxed_slice(),
                    }];
                    
                    if can_blover {
                        high_solutions.push(vec![
                            Unlockable::Blower,
                        ].into_boxed_slice());
                    }
                    
                    if !self.is_any_solution_satisfied(&high_solutions.into_boxed_slice(), level, &mut used_solutions) {
                        let threshold = if self.is_any_solution_satisfied(&low_solutions.into_boxed_slice(), level, &mut used_solutions) {
                            high_threshold
                        } else {
                            low_threshold
                        };
                        
                        threshold_table[zombie_idx as usize] = threshold_table[zombie_idx as usize].min(threshold);
                    }
                }
            }
            
            let mut removed_zombies: FxHashSet<u32> = HashSet::with_capacity_and_hasher(16, BuildHasherDefault::default());
            let mut new_spawns: Vec<(u32, u32)> = Vec::with_capacity(16);
            let one_eigth_curved = Self::weight_curve(0x1FFF_FFFF);
            for (i, (threshold, zombie)) in threshold_table.iter().zip(zombie_data.iter()).enumerate() {
                if let Some(weight) = spawns_map.get(&(i as u32)) {
                    let (max_freq, _) = spawn_data.max_frequency.get(&(i as u32)).unwrap();
                    let new_val = (*threshold / *max_freq) as f64 * *weight as f64 / zombie.default_weight as f64;
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
                let threshold     = threshold_table[i as usize];
                let zombie        = &zombie_data[i as usize];
                let (max_freq, _) = new_frequencies.max_frequency.get(&i).unwrap();
                let new_val       = (threshold / *max_freq) as f64 * weight as f64 / zombie.default_weight as f64;
                let new_weight    = (new_val * zombie.default_weight as f64).round() as u32;
                let old_weight    = *spawns_map.get(&i).unwrap();
                
                if new_val <= one_eigth_curved {
                    removed_zombies.insert(i);
                } else if new_weight < old_weight {
                    let old_val = Self::weight_curve_inverse(old_weight as f64 / zombie.default_weight as f64);
                    let new_val = Self::weight_curve_inverse(new_val);
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
    ) -> usize {
        let mut possible_levels: SmallVec<[(usize,f64); 64]> = SmallVec::new();
        let mut impossible_levels: SmallVec<[usize; 64]> = SmallVec::new();
        let mut total_weight = 0f64;
        
        for level_idx in remaining_levels {
            let mut level_weight = 1f64;
            
            if !(blacklist_set.contains(&(*level_idx as u32)) && remaining_levels.len() > 15) &&
                match self.is_level_possible(*level_idx as u32, if cattail_girl {45 - remaining_levels.len() as u32} else {0}) {
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
                                for i in remove_idxs.iter().rev() {
                                    zombies.remove(*i);
                                }
                                restrictions_data.modified_level_spawns.insert(*level_idx, zombies);
                            }
                            ImpossibleReason::BadPlants(_plants) => {
                                possible = false;
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
        assert_ne!(possible_levels.len(), 0);
        assert_ne!(total_weight, 0f64);
        
        let val = rng.next_u32() as f64 / 4_294_967_296. * total_weight;
        let idx = possible_levels.partition_point(|(_, csum)| *csum <= val);
        possible_levels[idx - 1].0
    }
    
    fn assign_solutions(
        &mut self,
        possible_solutions: FxHashMap<Problem, Solutions>,
        predetermined_level_plants: &mut FxHashMap<u8, (Unlockable, f32)>,
        remaining_levels: &mut Vec<u8>,
        blacklist_set: &FxHashSet<u32>,
        rng: &mut ChaCha8Rng,
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
                    let level_idx = Self::pick_level(self, remaining_levels, predetermined_level_plants, blacklist_set, false, rng);
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
    
    pub fn restrictions(seed: u64, _meta: &IL2CppDumper, _fuse_data: &HashMap<u32,[u32;2]>) -> Self {
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        #[allow(static_mut_refs)]
        let level_data = unsafe {LEVEL_DATA.as_ref()}.unwrap();
        
        let mut level_rng   = ChaCha8Rng::seed_from_u64(seed ^ hash_str("level_rng"));
        let mut weights_rng = ChaCha8Rng::seed_from_u64(seed ^ hash_str("zombie_weights"));
        let mut plants_rng  = ChaCha8Rng::seed_from_u64(seed ^ hash_str("plant_order"));
        
        let mut ret = Self {
            level_order: Vec::with_capacity(45),
            plant_order: vec![0xFF; 48],
            weights: Some(Vec::new()),
            firerates: None,
            cooldowns: None,
            costs: None,
            spawns: Some(Vec::new()),
            restrictions_data: Some(RestrictionsData {
                frequency_cache: HashMap::default(),
                level_spawns: HashMap::default(),
                modified_level_spawns: HashMap::default(),
                _level_plants: HashMap::default(),
                unlocked_plants: HashSet::default(),
            }),
        };
        
        let mut remaining_levels: Vec<u8> = (2..=45).collect();
        ret.level_order.push(1);
        if let Some(weights) = ret.weights.as_mut() {
            weights.push(vec![0xA0, 0xF, 0, 0]);
        }
        if let Some(spawns) = ret.spawns.as_mut() {
            spawns.push(vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
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
                    let weight_mul = Self::weight_curve(weights_rng.next_u32());
                    vec.push((idx as u32, (weight_mul * zombie_data[idx].default_weight as f64).round() as u32));
                    if zombie_data[idx].is_odyssey {
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
        restrictions_data.unlocked_plants.insert(first_plant);
        let mut predetermined_level_plants: FxHashMap<u8, (Unlockable, f32)> = HashMap::default();
        predetermined_level_plants.insert(1, (first_plant, 999.0));
        
        let possible_solutions_part_1 = Self::get_solutions_1();
        let possible_solutions_part_2 = Self::get_solutions_2();
        let possible_solutions_part_3 = Self::get_solutions_3();
        
        ret.assign_solutions(possible_solutions_part_1, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng);
        ret.assign_solutions(possible_solutions_part_2, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng);
        ret.assign_solutions(possible_solutions_part_3, &mut predetermined_level_plants, &mut remaining_levels, &blacklist_set, &mut plants_rng);
        
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
            Unlockable::Blower,
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
        
        while !remaining_levels.is_empty() {
            let level_idx = Self::pick_level(&mut ret, &remaining_levels, &predetermined_level_plants, &blacklist_set, true, &mut level_rng);
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
                for (idx, weight) in actual_spawns {
                    for (i, byte) in weight.to_le_bytes().iter().enumerate() {
                        weights_vec[(idx as usize) * 4 + i] = *byte;
                    }
                    Self::xor_bit_in_bitfield(idx as usize, &mut spawns_bitfield);
                }
                if let Some(weights) = ret.weights.as_mut() {
                    weights.push(weights_vec);
                }
                if let Some(spawns) = ret.spawns.as_mut() {
                    spawns.push(spawns_bitfield);
                }
            } else {
                unreachable!()
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
