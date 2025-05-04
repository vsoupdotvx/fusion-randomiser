#![cfg_attr(target_os = "linux", feature(unix_socket_ancillary_data))]
use std::{collections::HashMap, env, path::PathBuf, sync::{mpsc::{self, Receiver, Sender}, Arc}, thread::{self, sleep, JoinHandle}, time::Duration};

use data::{init_defaults_from_dump, LevelType, LEVEL_DATA, ZOMBIE_DATA};
use eframe::egui::{self, Align, Context, RichText};
use egui_file_dialog::FileDialog;
use egui_plot::{Legend, Line, Plot};
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

enum AppState {
    Disconnected,
    OptionsMenu,
    InGame,
}

enum AsmEvent {
    Init,
    LevelInfo(LevelUiData),
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

struct LevelUiData {
    level: usize,
    zombies: Vec<u32>,
    wave_data: Vec<f32>,
}

struct App {
    state:         AppState,
    file_dialog:   FileDialog,
    fusion_data:   Option<FusionData>,
    level_ui_data: Option<LevelUiData>,
    submitted:     bool,
    cfg:           Cfg,
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
            level_ui_data: None,
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
        let ctxt = ctxt.clone();
        let (ptx, arx) = mpsc::channel();
        let (atx, prx) = mpsc::channel();
        let poll_thread = thread::spawn(move || {
            match FusionProcess::new(true) {
                Ok(fusion) => Self::poll_thread(ctxt, prx, ptx, fusion),
                Err(err) => {
                    ctxt.request_repaint();
                    println!("{err}");
                }
            }
        });
        self.fusion_data = Some(FusionData {
            poll_thread,
            atx,
            arx,
        });
        self.state = AppState::OptionsMenu;
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
        
        init_defaults_from_dump(&dumper);
        
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
        
        let level_addr   = *sym_tab.get("level_idx").unwrap();
        let wait_addr    = *sym_tab.get("stopped").unwrap();
        let mix_ptr_addr = *sym_tab.get("mix_data_ptr").unwrap();
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
                let rand_data = unsafe { rand_data.as_mut().unwrap_unchecked() };
                if let Some(cooldowns) = &rand_data.cooldowns {
                    fusion.write_memory(*sym_tab.get("plant_cd_table").unwrap(), &cooldowns[level_idx as usize]).unwrap();
                }
                if let Some(costs) = &rand_data.costs {
                    fusion.write_memory(*sym_tab.get("plant_cost_table").unwrap(), &costs[level_idx as usize]).unwrap();
                }
                let spawn_vec = if cfg.spawns_enabled {
                    if let Some(spawns) = &rand_data.spawns {
                        fusion.write_memory(*sym_tab.get("zombie_spawn_bitfield").unwrap(), &spawns[level_idx as usize]).unwrap();
                    } else {
                        unreachable!()
                    }
                    if let Some(freqs) = &rand_data.freqs {
                        fusion.write_memory(*sym_tab.get("zombie_freqs").unwrap(), &freqs[level_idx as usize]).unwrap();
                    } else {
                        unreachable!()
                    }
                    if let Some(weights) = &rand_data.weights {
                        fusion.write_memory(*sym_tab.get("zombie_weights").unwrap(), &weights[level_idx as usize]).unwrap();
                    }
                    let mut spawn_vec: Vec<(u32,u32)> = Vec::new();
                    let spawns = &rand_data.spawns.as_ref().unwrap()[level_idx as usize];
                    for (i, bytes) in rand_data.weights.as_ref().unwrap()[level_idx as usize].chunks_exact(4).enumerate() {
                        if spawns[i >> 3] & (1 << (i & 7)) != 0 {
                            spawn_vec.push((i as u32, u32::from_le_bytes(bytes.try_into().unwrap())));
                        }
                    }
                    
                    spawn_vec
                } else {
                    Vec::new() //TODO
                };
                
                if let Some(firerates) = &rand_data.firerates {
                    fusion.write_memory(*sym_tab.get("plant_firerate_table").unwrap(), &firerates[level_idx as usize]).unwrap();
                }
                
                let zombie_data = ZOMBIE_DATA.get().unwrap();
                let freq_data = rand_data.compute_zombie_freq_data_cached(&spawn_vec, rand_data.level_order[level_idx as usize] as usize).unwrap();
                let mut zombies: Vec<u32> = spawn_vec.into_iter().map(|(id, _)| id).collect();
                let wave_data = freq_data.raw_averages;
                zombies.sort_by_key(|idx| zombie_data[*idx as usize].default_points);
                
                ptx.send(AsmEvent::LevelInfo(LevelUiData {
                    level: rand_data.level_order[level_idx as usize] as usize,
                    zombies,
                    wave_data,
                })).unwrap();
            }
            
            fusion.write_memory(wait_addr, &[0]).unwrap();
        }
        println!("Closed on level {}", level_idx + 1);
    }
    
    fn zombie_data_line<'a>(&self, idx: usize, idx_idx: usize) -> Line<'a> {
        let ui_data = self.level_ui_data.as_ref().unwrap();
        let points: Vec<[f64;2]> = ui_data.wave_data
            .iter()
            .skip(idx_idx)
            .step_by(ui_data.zombies.len())
            .enumerate()
            .map(|(x, y)| [(x + 1) as f64, *y as f64])
            .collect();
        let zombie_data = ZOMBIE_DATA.get().unwrap();
        Line::new(points)
            .name(zombie_data[idx as usize].name)
            .width(5.)
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
                if let Ok(msg) = data.arx.try_recv() {
                    match msg {
                        AsmEvent::Init => {
                            self.state = AppState::InGame;
                        }
                        AsmEvent::LevelInfo(info) => {
                            self.level_ui_data = Some(info);
                        }
                    }
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
                    ui.horizontal_wrapped(|ui| {
                        if self.submitted {ui.disable();}
                        let layout = ui.layout().clone().with_main_justify(true).with_main_align(Align::Min);
                        let size: egui::Vec2 = [135.0, 20.0].into();
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.firerates_enabled, "Random firerates").on_hover_ui(|ui| {
                                ui.label("Random firerates randomises the firerates of plants.
This will display a number between 1 and 9 on the seed packet, with 1 being the worst firerate and 9 being the best.
If both this and random cooldowns are enabled, this will be the first number.
The range of randomisation is between halved firerate and doubled firerate.
For fusions of plants, the firerate is the average of it's parents.");
                            });
                        });
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.costs_enabled, "Random costs").on_hover_ui(|ui| {
                                ui.label("Random costs randomises the costs of plants.
The range of randomisation is between halved cost and doubled cost.");
                            });
                            
                        });
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.cooldowns_enabled, "Random cooldowns").on_hover_ui(|ui| {
                                ui.label("Random cooldowns randomises the cooldowns of plants.
This will display a number between 1 and 9 on the seed packet, with 1 being the worst cooldown and 9 being the best.
If both this and random firerates are enabled, this will be the second number.
The range of randomisation is between halved cooldown and doubled cooldown.");
                            });
                            
                        });
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.spawns_enabled, "Random zombies").on_hover_ui(|ui| {
                                ui.label("Random zombies randomises which zombies spawn in each level and their weights (the likelihood of each type of zombie to spawn).
Most of the really hard zombies are only allowed to spawn starting on level 31.
The range of randomisation is between 0.1x weight and 10x weight, with heavy bias towards changing less.");
                            });
                            
                        });
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.tweaks_enabled, "Balance tweaks").on_hover_ui(|ui| {
                                ui.label("Balance tweaks changes certain things for the balance of the randomiser.
This currently does nothing except makes the entire almanac unlocked from the beginning.
More will likely be added in the future.");
                            });
                            
                        });
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            ui.checkbox(&mut self.cfg.restrictions, "Restrictions").on_hover_ui(|ui| {
                                ui.label("Restrictions adds restrictions on the randomisation to make the game possible to beat.
Without restrictions, you can get water levels with no water solutions, 4 flag roof levels with no pot, dark michael spam on your second level, etc.
Highly recommended.");
                            });
                            
                        });
                    });
                });
            }
            
            AppState::InGame => {
                egui::CentralPanel::default().show(ctxt, |ui| {
                    ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 20.;
                    if let Some(level_ui_data) = self.level_ui_data.as_ref() {
                        let level_data = LEVEL_DATA.get().unwrap();
                        let level_type = level_data[level_ui_data.level - 1].level_type;
                        let world = match level_type {
                            LevelType::Day => 1,
                            LevelType::Night => 2,
                            LevelType::Pool => 3,
                            LevelType::Fog => 4,
                            LevelType::Roof => 5,
                        };
                        let mut stage = 1;
                        for level in level_data[0 .. level_ui_data.level - 1].iter().rev() {
                            if level.level_type != level_type {
                                break;
                            }
                            stage += 1;
                        }
                        ui.label(RichText::new(format!("{}-{}", world, stage)).size(56.));
                        
                        let legend = Legend::default()
                            .position(egui_plot::Corner::LeftTop);
                        
                        let plot = Plot::new("Wave composition")
                            .legend(legend)
                            .show_axes(true)
                            .show_grid(true)
                            .allow_zoom(true);
                        
                        plot.show(ui, |plot_ui| {
                            for (idx_idx, idx) in level_ui_data.zombies.iter().enumerate() {
                                let line = self.zombie_data_line(*idx as usize, idx_idx);
                                plot_ui.line(line);
                            }
                        });
                    }
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
    native_options.viewport = native_options.viewport.with_min_inner_size([800., 600.]);
    eframe::run_native(
        concat!("PVZ Fusion Randomiser ", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc))))
    ).unwrap();
}
