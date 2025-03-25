#![cfg_attr(target_os = "linux", feature(unix_socket_ancillary_data))]
use std::{collections::HashMap, env, path::PathBuf, sync::{mpsc::{self, Receiver, Sender}, Arc}, thread::{self, sleep, JoinHandle}, time::Duration};

use data::init_defaults_from_dump;
use eframe::egui::{self, Align, Context, RichText};
use egui_file_dialog::FileDialog;
use fxhash::FxHashMap;
use il2cppdump::IL2CppDumper;
use logic::RandomisationData;
use patcher::Patch;
use process::FusionProcess;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use util::{hash_str, CommonError};

pub mod il2cppdump;
pub mod patcher;
pub mod process;
pub mod util;
pub mod data;
pub mod logic;
pub mod fast_map;

enum AppState {
    Disconnected,
    OptionsMenu,
}

enum AsmEvent {
    Init,
}

enum AppEvent {
    Conf(Cfg),
    Die,
    Dump(PathBuf),
    Ping,
}

#[derive(Clone)]
struct Cfg {
    firerates_enabled: bool,
    costs_enabled:     bool,
    cooldowns_enabled: bool,
    spawns_enabled:    bool,
    tweaks_enabled:    bool,
    restrictions:      bool,
    seed:            String,
}

struct FusionData {
    poll_thread: JoinHandle<()>,
    atx:         Sender<AppEvent>,
    arx:         Receiver<AsmEvent>,
}

struct App {
    state:       AppState,
    file_dialog: FileDialog,
    fusion_data: Option<FusionData>,
    submitted:   bool,
    cfg:         Cfg,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut seed_rng = ChaCha8Rng::from_os_rng();
        let seed = seed_rng.next_u64().to_string();
        
        let mut app = Self {
            state: AppState::Disconnected,
            file_dialog: FileDialog::new(),
            fusion_data: None,
            submitted:  false,
            cfg: Cfg {
                firerates_enabled: true,
                costs_enabled: true,
                cooldowns_enabled: true,
                spawns_enabled: true,
                tweaks_enabled: true,
                restrictions:   true,
                seed,
            },
        };
        
        app.attempt_to_find_fusion(&cc.egui_ctx);
        
        app
    }
    
    fn attempt_to_find_fusion(&mut self, ctxt: &Context) {
        if let Ok(fusion) = FusionProcess::new(true) {
            let ctxt = ctxt.clone();
            let (ptx, arx) = mpsc::channel();
            let (atx, prx) = mpsc::channel();
            let poll_thread = thread::spawn(move || {
                Self::poll_thread(ctxt, prx, ptx, fusion);
            });
            self.fusion_data = Some(FusionData {
                poll_thread,
                atx,
                arx,
            });
            self.state = AppState::OptionsMenu;
        }
    }
    
    fn try_send_to_poll_thread(&mut self, event: AppEvent) {
        if let Some(fusion_data) = self.fusion_data.as_mut() {
            if let Err(_) = fusion_data.atx.send(event) {
                self.state = AppState::Disconnected;
                self.submitted = false;
                self.fusion_data = None;
            }
        }
    }
    
    fn poll_thread(ctxt: Context, prx: Receiver<AppEvent>, ptx: Sender<AsmEvent>, mut fusion: FusionProcess) {
        let mut dumper = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
        init_defaults_from_dump(&dumper);
        
        let base_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/base.o"))).unwrap();
        let tutorials_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/tutorials.o"))).unwrap();
        let firerates_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/firerates.o"))).unwrap();
        let cost_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cost.o"))).unwrap();
        let cooldowns_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cooldowns.o"))).unwrap();
        let spawns_patch    = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/spawns.o"))).unwrap();
        let tweaks_patch    = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/tweaks.o"))).unwrap();
        
        let mut cfg = Cfg {
            firerates_enabled: false,
            costs_enabled:     false,
            cooldowns_enabled: false,
            spawns_enabled:    false,
            tweaks_enabled:    false,
            restrictions:      false,
            seed:     "".to_string(),
        };
        
        for event in prx.iter() {
            match event {
                AppEvent::Conf(new_cfg) => {
                    cfg = new_cfg;
                    break;
                },
                AppEvent::Die => return,
                AppEvent::Dump(path) => {
                    let dump_arc = Arc::new(dumper);
                    
                    match dump_arc.output_disasm(path.clone().join("out.s")) {
                        Ok(())  => println!("Successfully output disasm"),
                        Err(()) => println!("Failed to output disasm"),
                    }
                    
                    match dump_arc.output_structs(path.clone().join("out_structs.rs")) {
                        Ok(())  => println!("Successfully output structs"),
                        Err(()) => println!("Failed to output structs"),
                    }
                    
                    dumper = Arc::into_inner(dump_arc).unwrap();
                }
                AppEvent::Ping => {
                    
                }
            }
        }
        
        let sym_tab: FxHashMap<String, u64> = Patch::apply_patches(&[
            base_patch,
            tutorials_patch,
            firerates_patch,
            cost_patch,
            cooldowns_patch,
            spawns_patch,
            tweaks_patch,
        ], &dumper, &mut fusion);
        
        println!("Game patched!");
        
        let mut level_idx = 0;
        
        let level_addr    = *sym_tab.get("level_idx").unwrap();
        let wait_addr     = *sym_tab.get("stopped").unwrap();
        let mix_ptr_addr  = *sym_tab.get("mix_data_ptr").unwrap();
        //let game_ptr_addr = *sym_tab.get("game_app_ptr").unwrap();
        let mut initialized  = false;
        let mut mem_read_vec = Vec::new();
        let mut fuse_map: FxHashMap<u32,[u32;2]> = HashMap::default();
        let mut rand_data: Option<RandomisationData> = None;
        loop {
            sleep(Duration::from_millis(10));
            loop {
                if let Ok(msg) = prx.try_recv() {
                    match msg {
                        AppEvent::Conf(_new_cfg) => {
                            panic!("Config recieved at wrong time!")
                        },
                        AppEvent::Die => return,
                        AppEvent::Dump(path) => {
                            let dump_arc = Arc::new(dumper);
                            
                            match dump_arc.output_disasm(path.clone().join("out.s")) {
                                Ok(())  => println!("Successfully output disasm"),
                                Err(()) => println!("Failed to output disasm"),
                            }
                            
                            match dump_arc.output_structs(path.clone().join("out_structs.rs")) {
                                Ok(())  => println!("Successfully output structs"),
                                Err(()) => println!("Failed to output structs"),
                            }
                            
                            dumper = Arc::into_inner(dump_arc).unwrap();
                        }
                        AppEvent::Ping => {}
                    }
                } else {
                    break;
                }
            }
            if let Err(err) = fusion.read_memory(wait_addr, 1, &mut mem_read_vec) {
                match err.downcast::<CommonError>() {
                    Err(err) => panic!("Failed to read memory: {err}"),
                    _ => break, //if a CommonError is returned, it means fusion closed
                }
            }
            if mem_read_vec[0] == 0 {
                continue;
            }
            
            fusion.read_memory(level_addr, 4, &mut mem_read_vec).unwrap();
            level_idx = u32::from_le_bytes(mem_read_vec[0..4].try_into().unwrap());
            
            if !initialized {
                //fusion.read_memory(game_ptr_addr, 8, &mut mem_read_vec).unwrap();
                //let game_app_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
                
                //fusion.read_memory(, , )
                ptx.send(AsmEvent::Init).unwrap();
                ctxt.request_repaint();
                
                fusion.read_memory(mix_ptr_addr, 8, &mut mem_read_vec).unwrap();
                let mix_data_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
                
                fusion.read_memory(mix_data_addr + 0x10, 8, &mut mem_read_vec).unwrap();
                let mix_array_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
                
                fusion.read_memory(mix_array_addr, 0x20, &mut mem_read_vec).unwrap();
                let mix_array_width = u32::from_le_bytes(mem_read_vec[0x18..0x1C].try_into().unwrap()) as usize;
                
                fusion.read_memory(mix_array_addr + 0x20, mix_array_width * 8, &mut mem_read_vec).unwrap();
                
                let mut tmp_read_vec = Vec::new();
                for (i, bytes) in mem_read_vec.chunks_exact(8).enumerate() {
                    let mix_ptr = u64::from_le_bytes(bytes.try_into().unwrap());
                    if mix_ptr != 0 {
                        fusion.read_memory(mix_ptr + 0x10, 8, &mut tmp_read_vec).unwrap();
                        let plant_1 = u32::from_le_bytes(tmp_read_vec[0..4].try_into().unwrap());
                        let plant_2 = u32::from_le_bytes(tmp_read_vec[4..8].try_into().unwrap());
                        if plant_1 as usize != i && plant_2 as usize != i {
                            //println!("{i}: {} + {}", plant_1, plant_2);
                            fuse_map.insert(i as u32, [plant_1, plant_2]);
                        }
                    }
                }
                
                rand_data = Some(if cfg.restrictions {
                    RandomisationData::restrictions(hash_str(&cfg.seed), &dumper, &fuse_map)
                } else {
                    RandomisationData::no_restrictions(hash_str(&cfg.seed), &dumper, &fuse_map)
                });
                
                println!("Level order: {:?}", rand_data.as_ref().unwrap().level_order);
                
                fusion.write_memory(
                    *sym_tab.get("level_lut").unwrap(),
                    &unsafe { rand_data.as_ref().unwrap_unchecked() }.level_order,
                ).unwrap();
                
                fusion.write_memory(
                    *sym_tab.get("plant_lut").unwrap(),
                    &unsafe { rand_data.as_ref().unwrap_unchecked() }.plant_order,
                ).unwrap();
                initialized = true;
            }
            
            {
                let rand_data = unsafe { rand_data.as_ref().unwrap_unchecked() };
                if let Some(cooldowns) = &rand_data.cooldowns {
                    fusion.write_memory(*sym_tab.get("plant_cd_table").unwrap(), &cooldowns[level_idx as usize]).unwrap();
                }
                if let Some(costs) = &rand_data.costs {
                    fusion.write_memory(*sym_tab.get("plant_cost_table").unwrap(), &costs[level_idx as usize]).unwrap();
                }
                if let Some(spawns) = &rand_data.spawns {
                    fusion.write_memory(*sym_tab.get("zombie_spawn_bitfield").unwrap(), &spawns[level_idx as usize]).unwrap();
                }
                if let Some(freqs) = &rand_data.freqs {
                    fusion.write_memory(*sym_tab.get("zombie_freqs").unwrap(), &freqs[level_idx as usize]).unwrap();
                }
                if let Some(weights) = &rand_data.weights {
                    fusion.write_memory(*sym_tab.get("zombie_weights").unwrap(), &weights[level_idx as usize]).unwrap();
                }
                if let Some(firerates) = &rand_data.firerates {
                    fusion.write_memory(*sym_tab.get("plant_firerate_table").unwrap(), &firerates[level_idx as usize]).unwrap();
                }
            }
            
            fusion.write_memory(wait_addr, &[0]).unwrap();
        }
        println!("Closed on level {}", level_idx + 1);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(data) = self.fusion_data.take() {
            data.atx.send(AppEvent::Die).unwrap();
            
            data.poll_thread.join().unwrap();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctxt: &egui::Context, _frame: &mut eframe::Frame) {
        self.try_send_to_poll_thread(AppEvent::Ping);
        if let Some(data) = self.fusion_data.as_mut() {
            loop {
                if let Ok(_msg) = data.arx.try_recv() {
                    //match msg {
                    //    AsmEvent::Init => {
                    //        data.atx.send(AppEvent::Conf(self.cfg.clone())).unwrap();
                    //    }
                    //}
                } else {
                    break;
                }
            }
        }
        
        if let AppState::Disconnected = self.state {
            egui::TopBottomPanel::bottom("Disconnected").show(ctxt, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Failed to find running instance of PVZ Fusion",
                        )).size(24.)
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                        if ui.button(
                            RichText::new(format!(
                                "Retry",
                            )).size(24.)
                        ).clicked() {
                            self.attempt_to_find_fusion(ctxt);
                        }
                    });
                });
                ui.end_row();
            });
        }
        
        match self.state {
            AppState::Disconnected |
            AppState::OptionsMenu => {
                egui::CentralPanel::default().show(ctxt, |ui| {
                    ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 20.;
                    ui.horizontal(|ui| {
                        ui.label("Seed: ");
                        if self.submitted {
                            ui.text_edit_singleline(&mut self.cfg.seed.clone());
                        } else {
                            ui.text_edit_singleline(&mut self.cfg.seed);
                        }
                        if let AppState::OptionsMenu = self.state {
                            if ui.button("Dump").clicked() {
                                self.file_dialog.pick_directory();
                            }
                            if !self.submitted && ui.button("Submit").clicked() {
                                self.submitted = true;
                                self.try_send_to_poll_thread(AppEvent::Conf(self.cfg.clone()));
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_enabled(!self.submitted, egui::Checkbox::new(&mut self.cfg.firerates_enabled, "Random firerates"));
                        ui.add_enabled(!self.submitted, egui::Checkbox::new(&mut self.cfg.costs_enabled, "Random costs"));
                        ui.add_enabled(!self.submitted, egui::Checkbox::new(&mut self.cfg.cooldowns_enabled, "Random cooldowns"));
                        ui.add_enabled(!self.submitted, egui::Checkbox::new(&mut self.cfg.spawns_enabled, "Random zombies"));
                        ui.add_enabled(!self.submitted, egui::Checkbox::new(&mut self.cfg.tweaks_enabled, "Balance tweaks"));
                    });
                });
            }
        }
        
        self.file_dialog.update(ctxt);
        if let Some(path) = self.file_dialog.take_picked() {
            self.try_send_to_poll_thread(AppEvent::Dump(path));
        }
        ctxt.request_repaint_after_secs(0.5);
    }
}

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = native_options.viewport.with_min_inner_size([1024., 768.]);
    eframe::run_native(
        concat!("PVZ Fusion Randomiser ", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc))))
    ).unwrap();
    
    //let seed = 0;//hash_str(&seed_string);
    //println!("Seed: \"{seed_string}\"");
    
    //let mut fusion = loop {
    //    if let Ok(process) = FusionProcess::new(connect) {
    //        break process;
    //    }
    //    println!("Could not find a running instance of PVZ fusion, retrying in 5 seconds.");
    //    sleep(Duration::new(5, 0));
    //};
    //let mut dumper = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
    //
    //if let Some(out_dir) = out_dir {
    //    let out_path = match PathBuf::from_str(&out_dir) {
    //        Ok(path) => path,
    //        Err(err) => panic!("Error while parsing path: {err}")
    //    };
    //    if out_path.is_dir() {
    //        let dump_arc = Arc::new(dumper);
    //        
    //        match dump_arc.output_disasm(out_path.clone().join("out.s")) {
    //            Ok(())  => println!("Successfully output disasm"),
    //            Err(()) => println!("Failed to output disasm"),
    //        }
    //        
    //        match dump_arc.output_structs(out_path.clone().join("out_structs.rs")) {
    //            Ok(())  => println!("Successfully output structs"),
    //            Err(()) => println!("Failed to output structs"),
    //        }
    //        
    //        dumper = Arc::into_inner(dump_arc).unwrap();
    //    } else {
    //        panic!("Output path \"{}\" is not a directory!", out_path.to_string_lossy());
    //    }
    //}
    
    
    
}
