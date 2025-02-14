use std::collections::HashMap;

use rand::RngCore;
use rand_chacha::{rand_core::{SeedableRng, TryRngCore}, ChaCha8Rng};
use crate::{data::{LevelType, ZombieLanes, LEVEL_DATA}, il2cppdump::IL2CppDumper, util::hash_str};
use crate::data::ZOMBIE_DATA;

pub struct RandomisationData {
    pub level_order: Vec<u8>,
    pub plant_order: Vec<u8>,
    pub weights:     Option<Vec<Vec<u8>>>,
    pub firerates:   Option<Vec<Vec<u8>>>,
    pub cooldowns:   Option<Vec<Vec<u8>>>,
    pub costs:       Option<Vec<Vec<u8>>>,
    pub spawns:      Option<Vec<Vec<u8>>>,
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
        
        for (level_true_idx, level_idx) in (1..45).zip(level_order.iter().skip(1)) {
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
                    seed ^ hash_str(&level_idx.to_string()),
                    *level_idx as usize,
                    level_true_idx,
                )
            );
        }
        
        Self {
            level_order,
            plant_order,
            weights:   Some(weights),
            firerates: Some(firerates),
            cooldowns: Some(cooldowns),
            costs:     Some(costs),
            spawns:    Some(spawns),
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
    
    fn randomise_weights_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("zombie_weights")));
        #[allow(static_mut_refs)]
        let mut ret = Vec::with_capacity(unsafe {ZOMBIE_DATA.as_ref()}.unwrap().len() * 4);
        
        #[allow(static_mut_refs)]
        for zombie in unsafe {ZOMBIE_DATA.as_ref()}.unwrap() {
            let num = rng.try_next_u32().unwrap() as f64 / u32::MAX as f64;
            let weight_mul = 10.0f64.powf(((64. / 15. * num - 32. / 5.) * num + 62. / 15.) * num - 1.);
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
        
        fn xor_bit_in_bitfield(bit: usize, bitfield: &mut [u8]) {
            bitfield[bit >> 3] ^= 1 << (bit & 7) as u8;
        }
        
        for zombie_type in &level.default_zombie_types {
            xor_bit_in_bitfield(*zombie_type as usize, &mut ret);
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
            if (!zombie.is_odyssey && val >= u32::MAX / 20) ||
                (zombie.is_odyssey && val >= u32::MAX / 60) {
                continue;
            }
            
            if zombie.default_weight != 0 {
                xor_bit_in_bitfield(i, &mut ret);
            }
        }
        
        ret
    }
    
    fn randomise_level_order_no_restrictions(seed: u64) -> Vec<u8> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(hash_str("level_order")));
        
        let mut ret = vec![0u8; 45];
        let mut level_vec = Vec::with_capacity(45);
        
        level_vec.push((1, 0));
        
        for i in 2..46 {
            level_vec.push((i, rng.next_u32()));
        }
        
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
}
