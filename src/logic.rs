use std::collections::{HashMap, HashSet};

use fxhash::{FxHashMap, FxHashSet};
use rand::RngCore;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use crate::{data::{LevelType, ZombieLanes, LEVEL_DATA}, il2cppdump::IL2CppDumper, util::hash_str};
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
    frequency_cache: HashMap<FrequencyCacheKey, FrequencyData>,
    level_spawns: FxHashMap<u8, Vec<(u32,u32)>>,
    unlocked_plants: FxHashSet<String>,
}

#[allow(dead_code)]
enum ImpossibleReason {
    NoWaterSolution,
    InsufficientWaterSolution,
    FourFlag,
    HardZombies(Vec<(u32, u32)>),
    BadPlants(Vec<(u8,u8)>),
}

#[allow(dead_code)]
#[derive(Clone)]
struct FrequencyData {
    raw_averages: Vec<f32>,
    max_frequency: HashMap<u32, (f32, u32)>,
    first_flag_totals: HashMap<u32, f32>,
    first_wave_occurence_avgs: HashMap<u32, u32>,
}

#[derive(Hash, Clone, Eq, PartialEq)]
struct FrequencyCacheKey {
    spawns: Box<[u8]>,
    weights: Box<[u8]>,
    level: usize,
}

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
            Self::compute_zombie_freq_data(&spawns[level_true_idx - 1], &weights[level_true_idx - 1], *level_idx as usize).unwrap();
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
        let mut enum_variants: HashMap<String, u64> = HashMap::with_capacity(16384);
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
        let mut plant_lookup: HashMap<u32,u32> = HashMap::with_capacity(plant_ids.len());
        
        for (i, plant_id) in plant_ids.iter().enumerate() {
            plant_lookup.insert(*plant_id, i as u32);
        }
        
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
                ret[i] = get_fused_firerate(*fuse_plants, &plant_lookup, fuse_data, &ret, 0)
            }
        }
        
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
    
    fn compute_zombie_freq_data(spawns: &[u8], weights: &[u8], level: usize) -> Option<FrequencyData> {
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        #[allow(static_mut_refs)]
        let level_data = unsafe {LEVEL_DATA.as_ref()}.unwrap();
        
        let mut spawn_vec: Vec<(u32, u32, u32)> = Vec::with_capacity(weights.len() >> 2);
        for (i, bytes) in weights.chunks_exact(4).enumerate() {
            if spawns[i >> 3] & (1 << (i & 7)) != 0 {
                spawn_vec.push((i as u32, u32::from_le_bytes(bytes.try_into().unwrap()), zombie_data[i].default_points));
            }
        }
        
        spawn_vec.sort_by_key(|(_, _, points)| *points);
        let mut spawn_vec_pre_10 = spawn_vec.clone();
        let mut pre_10_map = Vec::from_iter(0..spawn_vec.len());
        
        for (i, (idx, _, _)) in spawn_vec.iter().enumerate().rev() {
            if zombie_data[*idx as usize].is_elite {
                spawn_vec_pre_10.remove(i);
                pre_10_map.remove(i);
            }
        }
        
        fn compute_freq_for_wave(spawn_vec: &Vec<(u32, u32, u32)>, wave: isize) -> Vec<f64> {
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
            
            let mut odds_old: FxHashMap<Box<[u16]>, (f64, isize)> =
                vec![(vec![0u16; spawn_vec.len()].into_boxed_slice(), (1f64, wave * 5 / 3))].into_iter().collect();
            let mut zombie_odds: Vec<f64> = vec![0f64; spawn_vec.len()];
            let mut loss_mul = 1f64;
            
            loop {
                let mut odds: FxHashMap<Box<[u16]>, (f64, isize)> = HashMap::default();
                
                for (zombies, (chance_1, remaining_points)) in odds_old {
                    for (i, chance_2) in spawns_lut[usize::min(remaining_points as usize, spawns_lut.len()) - 1].iter().enumerate() {
                        let chance = chance_1 * *chance_2;
                        let choice_wavepoints = spawn_vec[i].2 as isize;
                        let mut zombies = zombies.clone();
                        zombies[i] += 1;
                        if remaining_points > choice_wavepoints {
                            if chance > 1f64 / 16_777_216f64 {
                                odds.entry(zombies)
                                    .and_modify(|(odds, _)| *odds += chance)
                                    .or_insert((chance, remaining_points - choice_wavepoints));
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
            let spawn_data = if *wave < 10 {&spawn_vec_pre_10} else {&spawn_vec};
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
        //TODO: final wave additional zombies
        
        let mut max_frequency = HashMap::new();
        let mut first_flag_totals = HashMap::new();
        let mut first_wave_occurence_avgs = HashMap::new();
        
        for (i, (id, _, _)) in spawn_vec.iter().enumerate() {
            let mut max_freq      = 0f32;
            let mut max_wave      = 1;
            let mut total_freq    = 0f32;
            let mut total_freq_ff = 0f32;
            let mut first_wave    = 0;
            for (freq, j) in freq_array.iter().skip(i).step_by(spawn_vec.len()).zip(1..) {
                if *freq > max_freq {
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
    
    #[allow(dead_code)]
    fn compute_zombie_freq_data_cached(&mut self, spawns: &[u8], weights: &[u8], level: usize) -> Option<FrequencyData> {
        let key = FrequencyCacheKey {
            spawns: spawns.into(),
            weights: weights.into(),
            level,
        };
        let frequency_cache = &mut self.restrictions_data.as_mut().unwrap().frequency_cache;
        if let Some(entry) = frequency_cache.get(&key) {
            Some(entry.clone())
        } else if let Some(freq_data) = Self::compute_zombie_freq_data(spawns, weights, level) {
            frequency_cache.insert(key, freq_data.clone());
            Some(freq_data)
        } else {
            None
        }
    }
    
    fn is_level_possible(&mut self, _level_idx: u32) -> Result<(),ImpossibleReason> {
        
        
        
        Ok(())
    }
    
    pub fn restrictions(seed: u64) -> Self {
        #[allow(static_mut_refs)]
        let zombie_data = unsafe {ZOMBIE_DATA.as_ref()}.unwrap();
        #[allow(static_mut_refs)]
        let level_data = unsafe {LEVEL_DATA.as_ref()}.unwrap();
        let mut level_rng   = ChaCha8Rng::seed_from_u64(seed ^ hash_str("level_rng"));
        let mut weights_rng = ChaCha8Rng::seed_from_u64(seed ^ hash_str("zombie_weights"));
        //let mut spawns_rng  = ChaCha8Rng::seed_from_u64(seed ^ hash_str("zombie_spawns"));
        let mut ret = Self {
            level_order: Vec::with_capacity(45),
            plant_order: vec![0; 48],
            weights: Some(Vec::new()),
            firerates: None,
            cooldowns: None,
            costs: None,
            spawns: Some(Vec::new()),
            restrictions_data: Some(RestrictionsData {
                frequency_cache: HashMap::new(),
                level_spawns: HashMap::default(),
                unlocked_plants: HashSet::default(),
            }),
        };
        
        let mut remaining_levels: Vec<u8> = (2..=45).collect();
        ret.level_order.push(1);
        let restrictions_data = ret.restrictions_data.as_mut().unwrap();
        
        let mut blacklist_vec: Vec<(u32, u32)> = Vec::with_capacity(32);
        let mut blacklist_set: HashSet<u32> = HashSet::with_capacity(15);
        for (i, level) in level_data.iter().enumerate().skip(1) {
            if let Some(flags) = level.flags {
                if !level.is_conveyor && flags > 1 {
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
                    let idx = bit_pos as usize + byte_idx;
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
        
        while !remaining_levels.is_empty() {
            let mut possible_levels: Vec<(usize,f64)> = Vec::with_capacity(32);
            let mut total_weight = 0f64;
            for level_idx in &remaining_levels {
                let level_weight = 1f64;
                
                if !(blacklist_set.contains(&(*level_idx as u32)) && remaining_levels.len() > 15) && match ret.is_level_possible(*level_idx as u32) {
                    Ok(()) => true,
                    Err(reason) => match reason {
                        ImpossibleReason::NoWaterSolution |
                        ImpossibleReason::InsufficientWaterSolution |
                        ImpossibleReason::FourFlag => false,
                        ImpossibleReason::HardZombies(_zombies) => {
                            false
                        }
                        ImpossibleReason::BadPlants(_plants) => {
                            false
                        }
                    },
                } {
                    possible_levels.push((*level_idx as usize, total_weight));
                    total_weight += level_weight;
                }
            }
            assert_ne!(possible_levels.len(), 0);
            assert_ne!(total_weight, 0f64);
            
            let val = level_rng.next_u32() as f64 / 4_294_967_296. * total_weight;
            let idx = possible_levels.partition_point(|(_, csum)| *csum < val);
            let level_idx = possible_levels[idx].0;
            let level_idx_idx = remaining_levels.binary_search(&(level_idx as u8)).unwrap();
            let restrictions_data = ret.restrictions_data.as_mut().unwrap();
            
            if let Some(spawns) = restrictions_data.level_spawns.remove(&(level_idx as u8)) {
                let mut spawns_bitfield = vec![0; 16];
                let mut weights_vec = vec![0; zombie_data.len()];
                for (idx, weight) in spawns {
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
            
            //if remaining_levels.len() == 15 {
            //    for (level_idx, vec) in restrictions_data.level_spawns.iter_mut() {
            //        let level = &level_data[*level_idx as usize];
            //        for (i, zombie) in zombie_data.iter().enumerate() {
            //            if !zombie.is_odyssey {
            //                continue;
            //            }
            //            
            //            if let ZombieLanes::Water = zombie.allowed_lanes {
            //                match level.level_type {
            //                    LevelType::Pool |
            //                    LevelType::Fog => {}
            //                    _ => continue,
            //                }
            //            }
            //            
            //            if spawns_rng.next_u32() >= u32::MAX / 60 {
            //                continue;
            //            }
            //            
            //            let weight_mul = Self::weight_curve(weights_rng.next_u32());
            //            vec.push((i as u32, (weight_mul * zombie_data[idx].default_weight as f64).round() as u32));
            //        }
            //    }
            //}
        }
        
        ret
    }
}
