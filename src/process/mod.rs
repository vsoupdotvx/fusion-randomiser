use std::{fs::{read_dir, File, OpenOptions}, path::{Path, PathBuf}, pipe::{PipeReader, PipeWriter}};
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixStream;
use super::util::CommonError;
#[cfg(target_os = "linux")]
pub mod wine;

pub const PAGE_READWRITE:         u32 = 0x4;
pub const PAGE_EXECUTE_READ:      u32 = 0x20;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

#[allow(dead_code)]
pub struct FusionProcess {
    pub files_dir: PathBuf,
    pub dll_offset: u64,
    pub asm_offset: u64,
    fusion_handle: u32,
    #[cfg(target_os = "linux")]
    fusion_pid: i32,
    #[cfg(target_os = "linux")]
    wineserver_pid: i32,
    #[cfg(target_os = "linux")]
    mem: File,
    #[cfg(target_os = "linux")]
    wineserver_socket: UnixStream,
    #[cfg(target_os = "linux")]
    request_pipe: File,
    #[cfg(target_os = "linux")]
    reply_pipe_w: PipeWriter,
    #[cfg(target_os = "linux")]
    reply_pipe_r: PipeReader,
    #[cfg(target_os = "linux")]
    wait_pipe_w: PipeWriter,
    #[cfg(target_os = "linux")]
    wait_pipe_r: PipeReader,
    #[cfg(target_os = "linux")]
    request_offset_table: Vec<u32>,
}

impl FusionProcess {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        let ret = Self::find_process_linux();
        #[cfg(target_os = "windows")]
        let ret = Self::find_process_windows();
        ret
    }
    
    pub fn allocate_memory(&mut self, addr: u64, size: u64, prot: u32) {
        #[cfg(target_os = "linux")]
        self.allocate_memory_wine(addr, size, prot);
    }
    
    pub fn write_memory(&mut self, addr: u64, data: &[u8]) {
        #[cfg(target_os = "linux")]
        self.write_memory_linux(addr, data);
    }
    
    #[cfg(target_os = "windows")]
    fn find_process_windows() -> Result<Self, Box<dyn std::error::Error>> {
        Err(Box::new(CommonError::critical("Windows cannot currently find Plants Vs Zombies Fusion")))
    }
    
    #[cfg(target_os = "linux")]
    fn find_process_linux() -> Result<Self, Box<dyn std::error::Error>> {
        use std::{fs::{canonicalize, read_to_string}, io::{IoSliceMut, Read}, os::{fd::{AsRawFd, FromRawFd}, unix::{fs::FileExt, net::{AncillaryData, SocketAncillary, UnixStream}}}, pipe};

        use object::{Object, ObjectSection};
        use smallvec::SmallVec;

        let mut fusion_pid = 0i32;
        
        for f in read_dir("/proc/")? {
            let dir_ent  = f?;
            let file_name = dir_ent.file_name().into_string().unwrap();
            if let Ok(current_pid) = file_name.parse::<i32>() {
                let comm_path = Path::new("/proc/")
                    .join(file_name)
                    .join("comm");
                let comm: String = read_to_string(comm_path)?;
                if comm == "PlantsVsZombies\n" {
                    fusion_pid = current_pid;
                }
            }
        }
        
        if fusion_pid == 0 {
            return Err(Box::new(CommonError::critical("Plants Vs Zombies Fusion not currently running or not found")));
        }
        
        let maps_path = Path::new("/proc/")
            .join(format!("{fusion_pid}"))
            .join("maps");
        
        let mem_path = Path::new("/proc/")
            .join(format!("{fusion_pid}"))
            .join("mem");
        let mem = OpenOptions::new()
            .read(true)
            .write(true)
            .open(mem_path)?;
        
        let wine_map_files_path = Path::new("/proc/")
            .join(format!("{fusion_pid}"))
            .join("map_files");
        
        let mut files_dir: Option<PathBuf> = None;
        
        for f in read_dir(wine_map_files_path.clone())? {
            let dir_ent    = f?;
            let entry_name = dir_ent.file_name().into_string().unwrap();
            let entry_path = wine_map_files_path.clone().join(entry_name);
            if let Ok(true_path) = canonicalize(entry_path) {
                if let Some(map_file_name) = true_path.file_name() {
                    if map_file_name == "PlantsVsZombiesRH.exe" {
                        files_dir = Some(true_path.parent().unwrap().to_path_buf());
                        break;
                    }
                }
            }
        }
        
        if files_dir.is_none() {
            return Err(Box::new(CommonError::critical("Could not find installation directory!")));
        }
        
        let wine_exe_link_path = Path::new("/proc/")
            .join(format!("{fusion_pid}"))
            .join("exe");
        let mut wineserver_path = canonicalize(wine_exe_link_path)?;
        wineserver_path.set_file_name("wineserver");
        
        let mut wineserver_pid = 0i32;
        
        for f in read_dir("/proc/")? {
            let dent  = f?;
            let fname = dent.file_name().into_string().unwrap();
            if let Ok(current_pid) = fname.parse::<i32>() {
                let exe_path = Path::new("/proc/")
                    .join(fname)
                    .join("exe");
                if let Ok(exe) = canonicalize(exe_path) {
                    if exe == wineserver_path {
                        wineserver_pid = current_pid;
                    }
                }
            }
        }
        
        let wine_status_path = Path::new("/proc/")
            .join(format!("{wineserver_pid}"))
            .join("status");
        let wine_status = read_to_string(wine_status_path)?;
        let uid_line = wine_status.split('\n').nth(8).unwrap();
        let effective_uid_str = uid_line.split_whitespace().nth(2).unwrap();
        
        let wine_tmp_path = Path::new("/proc/")
            .join(format!("{wineserver_pid}"))
            .join("root") // If wine is being run in a chroot, we need to look at it's root instead for /tmp/
            .join("tmp")
            .join(format!(".wine-{}", effective_uid_str));
        
        let mut server_path_vec: SmallVec<[String;4]> = SmallVec::new();
        for f in read_dir(wine_tmp_path.clone())? {
            let dir_ent    = f?;
            let entry_name = dir_ent.file_name().into_string().unwrap();
            if entry_name.starts_with("server") {
                server_path_vec.push(entry_name);
            }
        }
        
        if server_path_vec.len() == 0 {
            return Err(Box::new(CommonError::critical("No wineservers found!")));
        } else if server_path_vec.len() > 1 {
            return Err(Box::new(CommonError::inconvenience("Multiple wineservers found!")));
        }
        
        let wineserver_socket_path = wine_tmp_path
            .join(&server_path_vec[0])
            .join("socket");
        
        let mut wineserver_socket = UnixStream::connect(wineserver_socket_path)?;
        
        let mut ancillary_buf = [0;256];
        let mut version_buf = [0;4];
        let mut ancillary = SocketAncillary::new(&mut ancillary_buf);
        
        wineserver_socket.recv_vectored_with_ancillary(
            &mut [IoSliceMut::new(&mut version_buf)],
            &mut ancillary
        )?;
        
        let mut request_fd: Option<i32> = None;
        
        for ancillary_result in ancillary.messages() {
            if let Ok(ancillary_data) = ancillary_result {
                if let AncillaryData::ScmRights(rights) = ancillary_data {
                    for fd in rights {
                        request_fd = Some(fd);
                    }
                }
            }
        }
        
        let mut request_pipe = unsafe {
            File::from_raw_fd(request_fd.ok_or(Box::new(CommonError::critical("No wineservers found!")))?)
        };
        
        let version = u32::from_ne_bytes(version_buf);
        if version < 786 {
            panic!("Wineserver protocol version is too old ({version} < 786)!\nTry upgrading wine to a version from 2024 or newer!")
        } else if version > 855 {
            panic!("Wineserver protocol version is too recent ({version} > 855)!\nTry downgrading wine or spam-pinging the developers on discord!")
        } else {
            println!("Wineserver protocol version: {version}");
        }
        
        let request_offset_table = wine::get_request_offset_table_for_version(version);
        
        let (mut reply_pipe_r, reply_pipe_w) = pipe::pipe()?;
        let (wait_pipe_r, wait_pipe_w)   = pipe::pipe()?;
        
        Self::send_fd_preinit(&mut wineserver_socket, reply_pipe_w.as_raw_fd());
        Self::send_fd_preinit(&mut wineserver_socket, wait_pipe_w.as_raw_fd());
        Self::init_first_thread(&mut request_pipe, &mut reply_pipe_r, &reply_pipe_w, &wait_pipe_w, &request_offset_table);
        
        let fusion_handle = Self::get_fusion_handle_preinit(&mut request_pipe, &mut reply_pipe_r, fusion_pid, &request_offset_table);
        
        let mut dll_file = File::open(files_dir.clone().unwrap().join("GameAssembly.dll"))?;
        let mut dll_data = vec![0; dll_file.metadata()?.len() as usize];
        dll_file.read(&mut dll_data)?;
        let dll_data = dll_data.into_boxed_slice();
        let dll_obj = object::File::parse(&*dll_data)?;
        let dll_text = dll_obj.section_by_name(".text").unwrap().data()?;
        let mut text_buf = vec![0u8; dll_text.len()];
        let mut dll_offset = None;
        let mut dll_text_end = None;
        
        let mappings_string = read_to_string(maps_path)?;
        let mapping_strings: Vec<&str> = mappings_string.split('\n').collect();
        let mut map_ranges: Vec<(u64, u64)> = Vec::with_capacity(mapping_strings.len() - 1);
        let mut check_next: Option<(u64, u64)> = None;
        
        for mapping_string in &mapping_strings {
            if mapping_string.is_empty() {
                continue;
            }
            let mapping_components: Vec<&str> = mapping_string.split_whitespace().collect();
            let (start_txt, end_txt) = mapping_components[0].split_once('-').unwrap();
            let start = u64::from_str_radix(start_txt, 16)?;
            let end = u64::from_str_radix(end_txt, 16)?;
            if let Some(mapping_start) = check_next {
                if mapping_components[1] == "r-xp" || mapping_components[1] == "r-xs" {
                    if let Ok(bytes_read) = mem.read_at(&mut text_buf, start) {
                        if bytes_read == dll_text.len() && text_buf == dll_text {
                            dll_offset = Some(mapping_start.0);
                            dll_text_end = Some(end);
                        }
                    }
                }
            }
            if mapping_components.len() > 5 {
                check_next = Some((start, end));
            } else {
                check_next = None
            }
            map_ranges.push((start, end));
        }
        
        let dll_offset   = dll_offset.expect("Failed to find GameAssembly.dll's mapping");
        let dll_text_end = dll_text_end.expect("Failed to find GameAssembly.dll's mapping");
        
        let start_idx = map_ranges.partition_point(|(_s, e)| {*e < dll_text_end - i32::MAX as u64});
        let mut asm_offset = (dll_text_end - i32::MAX as u64 - 1 + 0xFFFFF) & !0xFFFFF;
        
        for (original_start, original_end) in map_ranges.iter().skip(start_idx).take_while(|(_s, e)| {*e + 0xFFFFF <= dll_offset + i32::MAX as u64}) {
            let start = original_start & !0xFFFFF;
            let end = (original_end + 0xFFFFF) & !0xFFFFF;
            if (start..end).contains(&(asm_offset + 0xFFFFF)) || (start..end).contains(&asm_offset) {
                asm_offset = end;
            } else {
                break;
            }
        }
        
        println!("Fusion unix pid: {fusion_pid}");
        
        if let Some(fusion_handle) = fusion_handle {
            Ok(Self {
                fusion_handle,
                files_dir: files_dir.unwrap(),
                dll_offset,
                asm_offset,
                fusion_pid,
                wineserver_pid,
                mem,
                wineserver_socket,
                request_pipe,
                reply_pipe_w,
                reply_pipe_r,
                wait_pipe_w,
                wait_pipe_r,
                request_offset_table,
            })
        } else {
            Err(Box::new(CommonError::critical("Fusion handle could not be acquired!")))
        }
    }
    
    #[cfg(target_os = "linux")]
    pub fn write_memory_linux(&mut self, addr: u64, data: &[u8]) {
        use std::os::unix::fs::FileExt;
        self.mem.write_all_at(data, addr).unwrap();
    }
}
