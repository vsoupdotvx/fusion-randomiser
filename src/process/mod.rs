use std::{fs::File, io::Read, path::{Path, PathBuf}};
#[cfg(target_os = "linux")]
use std::{fs::{read_dir, OpenOptions, canonicalize, read_to_string}, io::IoSliceMut, os::unix::net::UnixStream, pipe::{PipeReader, PipeWriter}};
use object::{Object, ObjectSection};
#[cfg(target_os = "windows")]
use windows::{
    Wdk::System::SystemInformation::{
        NtQuerySystemInformation,
        SystemProcessInformation,
    },
    Win32::{
        Foundation::{
            HANDLE,
            HMODULE,
        },
        System::{
            Diagnostics::Debug::{
                ReadProcessMemory,
                WriteProcessMemory,
            },
            Memory::{
                MEMORY_BASIC_INFORMATION,
                MEM_COMMIT,
                MEM_FREE,
                MEM_RESERVE,
                PAGE_PROTECTION_FLAGS,
                VirtualAllocEx,
                VirtualQueryEx,
            },
            ProcessStatus::{
                EnumProcessModules,
                GetModuleFileNameExW,
                GetModuleInformation,
                MODULEINFO,
            },
            Threading::{
                OpenProcess,
                PROCESS_QUERY_INFORMATION,
                PROCESS_VM_OPERATION,
            },
            WindowsProgramming::SYSTEM_PROCESS_INFORMATION,
        },
    },
};
#[cfg(target_os = "windows")]
use windows_core::Free;
#[cfg(target_os = "windows")]
use core::{ffi::c_void, ptr};
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
    #[cfg(target_os = "windows")]
    fusion_handle: HANDLE,
    #[cfg(target_os = "linux")]
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
    pub fn new(_connect: bool) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        let ret = Self::find_process_linux(_connect);
        #[cfg(target_os = "windows")]
        let ret = Self::find_process_windows();
        ret
    }
    
    pub fn allocate_memory(&mut self, addr: u64, size: u64, prot: u32) {
        if size > 0 {
            #[cfg(target_os = "linux")]
            self.allocate_memory_wine(addr, size, prot);
            #[cfg(target_os = "windows")]
            self.allocate_memory_windows(addr, size, prot);
        }
    }
    
    pub fn write_memory(&mut self, addr: u64, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if !data.is_empty() {
            #[cfg(target_os = "linux")]
            self.write_memory_linux(addr, data)?;
            #[cfg(target_os = "windows")]
            self.write_memory_windows(addr, data)?;
        }
        Ok(())
    }
    
    pub fn read_memory(&mut self, addr: u64, len: usize, data: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        if len > 0 {
            #[cfg(target_os = "linux")]
            self.read_memory_linux(addr, len, data)?;
            #[cfg(target_os = "windows")]
            self.read_memory_windows(addr, len, data)?;
        }
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn find_process_windows() -> Result<Self, Box<dyn std::error::Error>> {
        use std::ptr::slice_from_raw_parts;
        
        let mut buf_size = 0u32;
        let mut buf: Vec<u8>;
        
        loop { //loop to prevent race conditions
            buf = vec![0; buf_size as usize];
            unsafe {
                if NtQuerySystemInformation(
                    SystemProcessInformation,
                    if buf.len() == 0 {
                        ptr::null_mut()
                    } else {
                        &mut buf[0] as *mut u8 as *mut c_void
                    },
                    buf_size,
                    &mut buf_size as *mut u32,
                ).is_ok() {break;}
            }
        }
        
        let mut pointer = ptr::addr_of!(buf[0]);
        let (fusion_pid, files_dir) = loop {
            let info = unsafe { &*(pointer as *const SYSTEM_PROCESS_INFORMATION) };
            
            let name = String::from_utf16_lossy(unsafe { &*slice_from_raw_parts(info.ImageName.Buffer.0, info.ImageName.Length as usize) });
            
            if name.ends_with("PlantsVsZombiesRH.exe") {
                break (info.UniqueProcessId, Path::new(&name).parent().unwrap().to_path_buf())
            }
            
            if info.NextEntryOffset == 0 {
                return Err(Box::new(CommonError::critical("Plants Vs Zombies Fusion not currently running or not found")));
            }
            pointer = pointer.wrapping_add(info.NextEntryOffset as usize);
        };
        
        let fusion_handle = match unsafe {
            OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION,
                false,
                fusion_pid.0 as usize as u32,
            )
        } {
            Ok(handle) => handle,
            Err(err) => return Err(Box::new(CommonError::critical(&format!("Error opening process: {err}")))),
        };
        
        buf_size = 0;
        let mut module_handles: Vec<HMODULE>;
        
        loop { //loop to prevent race conditions
            module_handles = vec![HMODULE::default(); buf_size as usize / size_of::<HMODULE>()];
            unsafe {
                if EnumProcessModules(
                    fusion_handle,
                    if module_handles.len() == 0 {
                        ptr::null_mut()
                    } else {
                        &mut module_handles[0] as *mut HMODULE
                    },
                    buf_size,
                    &mut buf_size as *mut u32,
                ).is_ok() {break;}
            }
        }
        
        let mut name_buf = [0u16; 2048];
        let mut game_assembly_dll_path = files_dir.clone();
        game_assembly_dll_path.push("GameAssembly.dll");
        let game_assembly_dll_path = game_assembly_dll_path.to_string_lossy().to_string();
        let mut game_assembly_module_info = None;
        
        for mut handle in module_handles {
            let name_len = unsafe { GetModuleFileNameExW(
                Some(fusion_handle),
                Some(handle),
                &mut name_buf,
            ) };
            if name_len > 0 {
                let name = String::from_utf16_lossy(&name_buf[0 .. name_len as usize]);
                if name == game_assembly_dll_path {
                    let mut module_info = MODULEINFO::default();
                    unsafe { GetModuleInformation(
                        fusion_handle,
                        handle,
                        &mut module_info as *mut MODULEINFO,
                        size_of::<MODULEINFO>() as u32,
                    ) }.expect("Failed to get module information");
                    
                    game_assembly_module_info = Some(module_info);
                }
            }
            unsafe { handle.free() };
        }
        
        let module_info = match game_assembly_module_info {
            Some(info) => info,
            None => return Err(Box::new(CommonError::critical("Could not find module GameAssembly.dll"))),
        };
        
        let dll_offset = module_info.lpBaseOfDll as usize as u64;
        
        let mut dll_file = File::open(files_dir.clone().join("GameAssembly.dll"))?;
        let mut dll_data = vec![0; dll_file.metadata()?.len() as usize];
        dll_file.read(&mut dll_data)?;
        let dll_data = dll_data.into_boxed_slice();
        let dll_obj = object::File::parse(&*dll_data)?;
        let dll_text_end = dll_obj.section_by_name(".rdata").unwrap().address() - dll_obj.relative_address_base() + dll_offset;
        
        let mut map_ranges: Vec<(u64,u64)> = Vec::new();
        
        let mut addr = 0x100000usize;
        loop {
            let mut lp_buffer = MEMORY_BASIC_INFORMATION::default();
            let retval = unsafe { VirtualQueryEx(
                fusion_handle,
                Some(addr as *const c_void),
                &mut lp_buffer as *mut MEMORY_BASIC_INFORMATION,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            ) };
            if retval == 0 {
                break;
            }
            if lp_buffer.State != MEM_FREE {
                map_ranges.push((lp_buffer.AllocationBase as usize as u64, lp_buffer.AllocationBase as usize as u64 + lp_buffer.RegionSize as u64));
            }
            addr = lp_buffer.AllocationBase as usize + lp_buffer.RegionSize;
        }
        
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
        
        Ok(Self {
            fusion_handle,
            files_dir,
            dll_offset,
            asm_offset,
        })
    }
    
    #[cfg(target_os = "windows")]
    pub fn write_memory_windows(&mut self, addr: u64, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut written = 0;
        unsafe {
            WriteProcessMemory(
                self.fusion_handle,
                addr as *const c_void,
                &data[0] as *const u8 as *const c_void,
                data.len(),
                Some(&mut written as *mut usize),
            )?;
        }
        if written != data.len() {
            Err(Box::new(CommonError::inconvenience("Wrong number of bytes were written")))
        } else {
            Ok(())
        }
    }
    
    #[cfg(target_os = "windows")]
    pub fn read_memory_windows(&mut self, addr: u64, len: usize, data: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let mut read = 0;
        data.resize(len, 0);
        unsafe {
            ReadProcessMemory(
                self.fusion_handle,
                addr as *const c_void,
                &mut data[0] as *mut u8 as *mut c_void,
                len,
                Some(&mut read as *mut usize),
            )?;
        }
        if read != len {
            Err(Box::new(CommonError::inconvenience("Wrong number of bytes were read")))
        } else {
            Ok(())
        }
    }
    
    #[cfg(target_os = "windows")]
    pub fn allocate_memory_windows(&mut self, addr: u64, size: u64, prot: u32) {
        unsafe {
            VirtualAllocEx(
                self.fusion_handle,
                Some(addr as *const c_void),
                size as usize,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_PROTECTION_FLAGS(prot),
            );
        }
    }
    
    #[cfg(target_os = "linux")]
    fn find_process_linux(_connect: bool) -> Result<Self, Box<dyn std::error::Error>> {
        use std::{os::{fd::{AsRawFd, FromRawFd}, unix::{fs::FileExt, net::{AncillaryData, SocketAncillary, UnixStream}}}, pipe};
        
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
        
        if server_path_vec.is_empty() {
            return Err(Box::new(CommonError::critical("No wineservers found!")));
        } else if server_path_vec.len() > 1 {
            return Err(Box::new(CommonError::inconvenience("Multiple wineservers found!")));
        }
        
        let wineserver_socket_path = wine_tmp_path
            .join(&server_path_vec[0])
            .join("socket");
        
        //let mut wineserver_socket = None;
        //let mut request_pipe = None;
        //let mut request_offset_table = None;
        let (mut reply_pipe_r, reply_pipe_w) = pipe::pipe()?;
        let (wait_pipe_r, wait_pipe_w)       = pipe::pipe()?;
        //let mut fusion_handle = Some(u32::MAX);
        
        let mut wineserver_socket = UnixStream::connect(wineserver_socket_path)?;
        
        let mut ancillary_buf = [0;256];
        let mut version_buf = [0;4];
        let mut ancillary = SocketAncillary::new(&mut ancillary_buf);
        
        wineserver_socket.recv_vectored_with_ancillary(
            &mut [IoSliceMut::new(&mut version_buf)],
            &mut ancillary
        )?;
        
        let mut request_fd: Option<i32> = None;
        
        for ancillary_data in ancillary.messages().flatten() {
            if let AncillaryData::ScmRights(rights) = ancillary_data {
                for fd in rights {
                    request_fd = Some(fd);
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
        
        Self::send_fd_preinit(&mut wineserver_socket, reply_pipe_w.as_raw_fd());
        Self::send_fd_preinit(&mut wineserver_socket, wait_pipe_w.as_raw_fd());
        Self::init_first_thread(&mut request_pipe, &mut reply_pipe_r, &reply_pipe_w, &wait_pipe_w, &request_offset_table);
        
        let fusion_handle = Self::get_fusion_handle_preinit(&mut request_pipe, &mut reply_pipe_r, fusion_pid, &request_offset_table);
        
        let mut dll_file = File::open(files_dir.clone().unwrap().join("GameAssembly.dll"))?;
        let mut dll_data = vec![0; dll_file.metadata()?.len() as usize];
        dll_file.read_exact(&mut dll_data)?;
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
        println!("Fusion addr: {dll_offset:X}, Asm addr: {asm_offset:X}");
        
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
    pub fn write_memory_linux(&mut self, addr: u64, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::FileExt;
        let written = self.mem.write_at(data, addr)?;
        if written != data.len() {
            Err(Box::new(CommonError::inconvenience("Wrong number of bytes were written")))
        } else {
            Ok(())
        }
    }
    
    #[cfg(target_os = "linux")]
    pub fn read_memory_linux(&mut self, addr: u64, len: usize, data: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::FileExt;
        data.resize(len, 0);
        let read = self.mem.read_at(&mut data[0 .. len], addr)?;
        if read != len {
            Err(Box::new(CommonError::inconvenience(&format!("Wrong number of bytes were read: {read} vs {len}"))))
        } else {
            Ok(())
        }
    }
}
