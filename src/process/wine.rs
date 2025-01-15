//786 is the oldest supported version (i.e. the last version before 2024)
//855 is the newest supported version
use core::slice;
use std::{
    fs::File,
    io::{
        IoSlice,
        IoSliceMut,
        Read,
        Write,
    },
    mem::{
        self,
        transmute,
    },
    os::{
        fd::AsRawFd,
        unix::net::{
            SocketAncillary,
            UnixStream,
        },
    },
    pipe::{
        PipeReader,
        PipeWriter,
    },
    process,
    ptr::slice_from_raw_parts,
    thread::sleep,
    time::Duration,
};

use gettid::gettid;

use super::FusionProcess;

#[derive(Debug)]
#[allow(dead_code)]
#[repr(C)]
enum Request
{
    ReqNewProcess = 0,
    ReqGetNewProcessInfo,
    ReqNewThread,
    ReqGetStartupInfo,
    ReqInitProcessDone,
    ReqInitFirstThread,
    ReqInitThread,
    ReqTerminateProcess,
    ReqTerminateThread,
    ReqGetProcessInfo,
    ReqGetProcessDebugInfo,
    ReqGetProcessImageName,
    ReqGetProcessVmCounters,
    ReqSetProcessInfo,
    ReqGetThreadInfo,
    ReqGetThreadTimes,
    ReqSetThreadInfo,
    ReqSuspendThread,
    ReqResumeThread,
    ReqQueueApc,
    ReqGetApcResult,
    ReqCloseHandle,
    ReqSetHandleInfo,
    ReqDupHandle,
    ReqAllocateReserveObject, //added in 843
    ReqCompareObjects,
    ReqMakeTemporary, //removed in 797
    ReqSetObjectPermanence, //added in 797
    ReqOpenProcess,
    ReqOpenThread,
    ReqSelect,
    ReqCreateEvent,
    ReqEventOp,
    ReqQueryEvent,
    ReqOpenEvent,
    ReqCreateKeyedEvent,
    ReqOpenKeyedEvent,
    ReqCreateMutex,
    ReqReleaseMutex,
    ReqOpenMutex,
    ReqQueryMutex,
    ReqCreateSemaphore,
    ReqReleaseSemaphore,
    ReqQuerySemaphore,
    ReqOpenSemaphore,
    ReqCreateFile,
    ReqOpenFileObject,
    ReqAllocFileHandle,
    ReqGetHandleUnixName,
    ReqGetHandleFd,
    ReqGetDirectoryCacheEntry,
    ReqFlush,
    ReqGetFileInfo,
    ReqGetVolumeInfo,
    ReqLockFile,
    ReqUnlockFile,
    ReqRecvSocket,
    ReqSendSocket,
    ReqSocketGetEvents,
    ReqSocketSendIcmpId,
    ReqSocketGetIcmpId,
    ReqGetNextConsoleRequest,
    ReqReadDirectoryChanges,
    ReqReadChange,
    ReqCreateMapping,
    ReqOpenMapping,
    ReqGetMappingInfo,
    ReqGetImageMapAddress,
    ReqMapView,
    ReqMapImageView,
    ReqMapBuiltinView,
    ReqGetImageViewInfo,
    ReqUnmapView,
    ReqGetMappingCommittedRange,
    ReqAddMappingCommittedRange,
    ReqIsSameMapping,
    ReqGetMappingFilename,
    ReqListProcesses,
    ReqCreateDebugObj,
    ReqWaitDebugEvent,
    ReqQueueExceptionEvent,
    ReqGetExceptionStatus,
    ReqContinueDebugEvent,
    ReqDebugProcess,
    ReqSetDebugObjInfo,
    ReqReadProcessMemory,
    ReqWriteProcessMemory,
    ReqCreateKey,
    ReqOpenKey,
    ReqDeleteKey,
    ReqFlushKey,
    ReqEnumKey,
    ReqSetKeyValue,
    ReqGetKeyValue,
    ReqEnumKeyValue,
    ReqDeleteKeyValue,
    ReqLoadRegistry,
    ReqUnloadRegistry,
    ReqSaveRegistry,
    ReqSetRegistryNotification,
    ReqRenameKey,
    ReqCreateTimer,
    ReqOpenTimer,
    ReqSetTimer,
    ReqCancelTimer,
    ReqGetTimerInfo,
    ReqGetThreadContext,
    ReqSetThreadContext,
    ReqGetSelectorEntry,
    ReqAddAtom,
    ReqDeleteAtom,
    ReqFindAtom,
    ReqGetAtomInformation,
    ReqGetMsgQueueHandle, //added in 818
    ReqGetMsgQueue,
    ReqSetQueueFd,
    ReqSetQueueMask,
    ReqGetQueueStatus,
    ReqGetProcessIdleEvent,
    ReqSendMessage,
    ReqPostQuitMessage,
    ReqSendHardwareMessage,
    ReqGetMessage,
    ReqReplyMessage,
    ReqAcceptHardwareMessage,
    ReqGetMessageReply,
    ReqSetWinTimer,
    ReqKillWinTimer,
    ReqIsWindowHung,
    ReqGetSerialInfo,
    ReqSetSerialInfo,
    ReqCancelSync,
    ReqRegisterAsync,
    ReqCancelAsync,
    ReqGetAsyncResult,
    ReqSetAsyncDirectResult,
    ReqRead,
    ReqWrite,
    ReqIoctl,
    ReqSetIrpResult,
    ReqCreateNamedPipe,
    ReqSetNamedPipeInfo,
    ReqCreateWindow,
    ReqDestroyWindow,
    ReqGetDesktopWindow,
    ReqSetWindowOwner,
    ReqGetWindowInfo,
    ReqSetWindowInfo,
    ReqSetParent,
    ReqGetWindowParents,
    ReqGetWindowList, //added in 850
    ReqGetClassWindows, //added in 851
    ReqGetWindowChildren, //removed in 852
    ReqGetWindowChildrenFromPoint,
    ReqGetWindowTree,
    ReqSetWindowPos,
    ReqGetWindowRectangles,
    ReqGetWindowText,
    ReqSetWindowText,
    ReqGetWindowsOffset,
    ReqGetVisibleRegion,
    ReqGetSurfaceRegion, //removed in 803
    ReqGetWindowRegion,
    ReqSetWindowRegion,
    ReqGetUpdateRegion,
    ReqUpdateWindowZorder,
    ReqRedrawWindow,
    ReqSetWindowProperty,
    ReqRemoveWindowProperty,
    ReqGetWindowProperty,
    ReqGetWindowProperties,
    ReqCreateWinstation,
    ReqOpenWinstation,
    ReqCloseWinstation,
    ReqSetWinstationMonitors, //added in 847
    ReqGetProcessWinstation,
    ReqSetProcessWinstation,
    ReqEnumWinstation,
    ReqCreateDesktop,
    ReqOpenDesktop,
    ReqOpenInputDesktop,
    ReqSetInputDesktop, //added in 794
    ReqCloseDesktop,
    ReqGetThreadDesktop,
    ReqSetThreadDesktop,
    ReqEnumDesktop, //removed in 849
    ReqSetUserObjectInfo,
    ReqRegisterHotkey,
    ReqUnregisterHotkey,
    ReqAttachThreadInput,
    ReqGetThreadInputData, //added in 823, removed in 830
    ReqGetThreadInput,
    ReqGetLastInputTime,
    ReqGetKeyState,
    ReqSetKeyState,
    ReqSetForegroundWindow,
    ReqSetFocusWindow,
    ReqSetActiveWindow,
    ReqSetCaptureWindow,
    ReqSetCaretWindow,
    ReqSetCaretInfo,
    ReqSetHook,
    ReqRemoveHook,
    ReqStartHookChain,
    ReqFinishHookChain,
    ReqGetHookInfo,
    ReqCreateClass,
    ReqDestroyClass,
    ReqSetClassInfo,
    ReqOpenClipboard,
    ReqCloseClipboard,
    ReqEmptyClipboard,
    ReqSetClipboardData,
    ReqGetClipboardData,
    ReqGetClipboardFormats,
    ReqEnumClipboardFormats,
    ReqReleaseClipboard,
    ReqGetClipboardInfo,
    ReqSetClipboardViewer,
    ReqAddClipboardListener,
    ReqRemoveClipboardListener,
    ReqCreateToken,
    ReqOpenToken,
    ReqSetGlobalWindows, //removed in 842
    ReqSetDesktopShellWindows, //added in 842
    ReqAdjustTokenPrivileges,
    ReqGetTokenPrivileges,
    ReqCheckTokenPrivileges,
    ReqDuplicateToken,
    ReqFilterToken,
    ReqAccessCheck,
    ReqGetTokenSid,
    ReqGetTokenGroups,
    ReqGetTokenDefaultDacl,
    ReqSetTokenDefaultDacl,
    ReqSetSecurityObject,
    ReqGetSecurityObject,
    ReqGetSystemHandles,
    ReqGetTcpConnections, //added in 839
    ReqGetUdpEndpoints, //added in 840
    ReqCreateMailslot,
    ReqSetMailslotInfo,
    ReqCreateDirectory,
    ReqOpenDirectory,
    ReqGetDirectoryEntries, //added in 805
    ReqGetDirectoryEntry, //removed in 806
    ReqCreateSymlink,
    ReqOpenSymlink,
    ReqQuerySymlink,
    ReqGetObjectInfo,
    ReqGetObjectName,
    ReqGetObjectType,
    ReqGetObjectTypes,
    ReqAllocateLocallyUniqueId,
    ReqCreateDeviceManager,
    ReqCreateDevice,
    ReqDeleteDevice,
    ReqGetNextDeviceRequest,
    ReqGetKernelObjectPtr,
    ReqSetKernelObjectPtr,
    ReqGrabKernelObject,
    ReqReleaseKernelObject,
    ReqGetKernelObjectHandle,
    ReqMakeProcessSystem,
    ReqGrantProcessAdminToken, //added in 853
    ReqGetTokenInfo,
    ReqCreateLinkedToken,
    ReqCreateCompletion,
    ReqOpenCompletion,
    ReqAddCompletion,
    ReqRemoveCompletion,
    ReqGetThreadCompletion, //added in 845
    ReqQueryCompletion,
    ReqSetCompletionInfo,
    ReqAddFdCompletion,
    ReqSetFdCompletionMode,
    ReqSetFdDispInfo,
    ReqSetFdNameInfo,
    ReqSetFdEofInfo,
    ReqGetWindowLayeredInfo,
    ReqSetWindowLayeredInfo,
    ReqAllocUserHandle,
    ReqFreeUserHandle,
    ReqSetCursor,
    ReqGetCursorHistory,
    ReqGetRawInputBuffer,
    ReqUpdateRawInputDevices,
    ReqCreateJob,
    ReqOpenJob,
    ReqAssignJob,
    ReqProcessInJob,
    ReqSetJobLimits,
    ReqSetJobCompletionPort,
    ReqGetJobInfo,
    ReqTerminateJob,
    ReqSuspendProcess,
    ReqResumeProcess,
    ReqGetNextThread,
    ReqSetKeyboardRepeat, //added in 804
    ReqNbRequests,
}

pub fn get_request_offset_table_for_version(version: u32) -> Vec<u32> {
    let mut table = vec![0; Request::ReqNbRequests as usize];
    for i in 1 .. Request::ReqNbRequests as i32 {
        let variant: Request = unsafe {transmute(i)};
        let range = match variant {
            Request::ReqAllocateReserveObject  => 843 .. u32::MAX, //added in 843
            Request::ReqMakeTemporary          => 0   .. 797,      //removed in 797
            Request::ReqSetObjectPermanence    => 797 .. u32::MAX, //added in 797
            Request::ReqGetMsgQueueHandle      => 818 .. u32::MAX, //added in 818
            Request::ReqGetWindowList          => 850 .. u32::MAX, //added in 850
            Request::ReqGetClassWindows        => 851 .. u32::MAX, //added in 851
            Request::ReqGetWindowChildren      => 0   .. 852,      //removed in 852
            Request::ReqGetSurfaceRegion       => 0   .. 803,      //removed in 803
            Request::ReqSetWinstationMonitors  => 847 .. u32::MAX, //added in 847
            Request::ReqSetInputDesktop        => 794 .. u32::MAX, //added in 794
            Request::ReqEnumDesktop            => 0   .. 849,      //removed in 849
            Request::ReqGetThreadInputData     => 823 .. 830,      //added in 823, removed in 830
            Request::ReqSetGlobalWindows       => 0   .. 842,      //removed in 842
            Request::ReqSetDesktopShellWindows => 842 .. u32::MAX, //added in 842
            Request::ReqGetTcpConnections      => 839 .. u32::MAX, //added in 839
            Request::ReqGetUdpEndpoints        => 840 .. u32::MAX, //added in 840
            Request::ReqGetDirectoryEntries    => 805 .. u32::MAX, //added in 805
            Request::ReqGetDirectoryEntry      => 0   .. 806,      //removed in 806
            Request::ReqGrantProcessAdminToken => 853 .. u32::MAX, //added in 853
            Request::ReqGetThreadCompletion    => 845 .. u32::MAX, //added in 845
            Request::ReqSetKeyboardRepeat      => 804 .. u32::MAX, //added in 804
            _                                  => 0   .. u32::MAX,
        };
        if !range.contains(&version) {
            for val in table.iter_mut().skip(i as usize) {
                *val += 1;
            }
        }
    }
    table
}

#[derive(Debug)]
#[allow(dead_code)]
#[repr(C)]
enum ApcType {
    ApcNone = 0,
    ApcUser,
    ApcAsyncIo,
    ApcVirtualAlloc,
    ApcVirtualAllocEx,
    ApcVirtualFree,
    ApcVirtualQuery,
    ApcVirtualProtect,
    ApcVirtualFlush,
    ApcVirtualLock,
    ApcVirtualUnlock,
    ApcMapView,
    ApcMapViewEx,
    ApcUnmapView,
    ApcCreateThread,
    ApcDupHandle,
}

#[derive(Debug)]
#[repr(C)]
struct RequestHeader {
    request:  Request,
    request_size: u32,
    reply_size:   u32,
}
#[derive(Debug)]
#[repr(C)]
struct ReplyHeader {
    error:      u32,
    reply_size: u32,
}

#[derive(Debug)]
#[repr(C)]
struct ListProcessesRequest {
    header: RequestHeader,
}
#[derive(Debug)]
#[repr(C)]
struct ListProcessesReply {
    header:     ReplyHeader,
    info_size:          u32,
    process_count:      i32,
    total_thread_count: i32,
    total_name_len:     u32,
}

#[derive(Debug)]
#[repr(C)]
struct QueueApcRequest {
    header: RequestHeader,
    handle: u32,
}
#[derive(Debug)]
#[repr(C)]
struct QueueApcReply {
    header: ReplyHeader,
    handle: u32,
    slf:    i32,
}
#[derive(Debug)]
#[repr(C)]
struct VirtualAllocEx {
    apc_type:   ApcType,
    op_type:    u32,
    addr:       u64,
    size:       u64,
    limit_low:  u64,
    limit_high: u64,
    align:      u64,
    prot:       u32,
    attributes: u32,
}

#[derive(Debug)]
#[repr(C)]
struct SendFd {
    tid: u32,
    fd:  i32,
}

#[derive(Debug)]
#[repr(C)]
struct InitFirstThreadRequest {
    header: RequestHeader,
    unix_pid:         i32,
    unix_tid:         i32,
    debug_level:      i32,
    reply_fd:         i32,
    wait_fd:          i32,
}
#[derive(Debug)]
#[repr(C)]
struct InitFirstThreadReply {
    header: ReplyHeader,
    pid:            i32,
    tid:            i32,
    server_start:   i64,
    session_id:     u32,
    info_size:      u32,
}

#[derive(Debug)]
#[repr(C)]
struct ProcessInfo
{
    start_time:   i64,
    name_len:     u32,
    thread_count: i32,
    priority:     i32,
    pid:          u32,
    parent_pid:   u32,
    session_id:   u32,
    handle_count: i32,
    unix_pid:     i32,
}
#[derive(Debug)]
#[repr(C)]
struct ThreadInfo
{
    start_time:       i64,
    tid:              u32,
    base_priority:    i32,
    current_priority: i32,
    unix_tid:         i32,
    teb:              i64,
    entry_point:      i64,
}

#[derive(Debug)]
#[repr(C)]
struct OpenProcessRequest {
    header: RequestHeader,
    pid:    u32,
    access: u32,
    attrs:  u32,
}
#[derive(Debug)]
#[repr(C)]
struct OpenProcessReply {
    header: ReplyHeader,
    handle: u32,
}

#[derive(Debug)]
#[repr(C)]
struct SelectRequest
{
    header: RequestHeader,
    flags:            i32,
    cookie:           i64,
    timeout:          i64,
    size:             u32,
    prev_apc:         u32,
}
#[derive(Debug)]
#[repr(C)]
struct SelectReply
{
    header: ReplyHeader,
    apc_handle:     u32,
    signaled:       i32,
}
#[derive(Debug)]
#[repr(C)]
struct WakeUpReply {
    cookie:   i64,
    signaled: i32,
    _pad:     i32,
}

#[derive(Debug)]
#[repr(C)]
struct GetApcResultRequest {
    header: RequestHeader,
    handle: u32,
}
#[derive(Debug)]
#[repr(C)]
struct GetApcResultReplyVirtualAllocEx {
    header: ReplyHeader,
    apc_type:   ApcType,
    status:         u32,
    addr:           i64,
    size:           i64,
    _pad:       [u8;16],
}

pub const MEM_COMMIT:  u32 = 0x1000;
pub const MEM_RESERVE: u32 = 0x2000;

impl FusionProcess {
    pub fn allocate_memory_wine(&mut self, addr: u64, size: u64, prot: u32) {
        let virtual_alloc_ex_apc = VirtualAllocEx {
            apc_type:   ApcType::ApcVirtualAllocEx,
            op_type:    MEM_COMMIT | MEM_RESERVE,
            addr,
            size,
            limit_low:  0,
            limit_high: 0,
            align:      0,
            prot,
            attributes: 0,
        };
        let apc_status = self.server_queue_process_apc(self.fusion_handle, &virtual_alloc_ex_apc);
        
        match apc_status {
            0 => {}
            0xC0000018 => panic!("Tried to map memory that was already mapped @0x{addr:X}, maybe you've already run the fusion randomiser?"),
            other => panic!("Wine VirtualAllocEx failed with status 0x{other:X}")
        }
    }
    
    fn server_queue_process_apc<T>(&mut self, process: u32, call: &T) -> u32 {
        loop {
            let mut apc_call = [0u8;64];
            let call_bytes   = unsafe {&*slice_from_raw_parts(&*(call as *const T as *const u8), size_of::<T>())};
            for (src, dst) in call_bytes.iter().zip(apc_call.iter_mut()) {
                *dst = *src;
            }
            let mut reply = unsafe {mem::zeroed::<QueueApcReply>()};
            
            let request = QueueApcRequest {
                header: RequestHeader { request: Request::ReqQueueApc, request_size: 64, reply_size: 0 },
                handle: process,
            };
            
            self.send_request(&request, Some(vec![&apc_call]));
            self.recv_reply(&mut reply, None);
            
            #[allow(unused_mut)]
            let mut cookie = 0;
            let mut data = [0u8;48];
            data[40] = 2; //SELECT_WAIT_ALL
            for (src, dst) in reply.handle.to_ne_bytes().iter().zip(data.iter_mut().skip(44)) {
                *dst = *src;
            }
            
            let mut apc_handle = 0;
            
            loop {
                sleep(Duration::from_millis(2));
                let request = SelectRequest {
                    header:   RequestHeader { request: Request::ReqSelect, request_size: 48, reply_size: 0 },
                    flags:    2,
                    cookie:   &cookie as *const i32 as i64,
                    timeout:  i64::MAX,
                    size:     8,
                    prev_apc: apc_handle,
                };
                let mut reply = unsafe {mem::zeroed::<SelectReply>()};
                let mut reply_data = Vec::new();
                
                self.send_request(&request, Some(vec![&data]));
                self.recv_reply(&mut reply, Some(&mut reply_data));
                
                if reply.signaled != 0 {
                    break;
                }
                
                apc_handle = reply.apc_handle;
                let mut wake_up_reply = unsafe {mem::zeroed::<WakeUpReply>()};
                self.recv_wait(&mut wake_up_reply);
            }
            
            let request = GetApcResultRequest {
                header: RequestHeader { request: Request::ReqGetApcResult, request_size: 0, reply_size: 0 },
                handle: reply.handle,
            };
            
            let mut reply = unsafe {mem::zeroed::<GetApcResultReplyVirtualAllocEx>()};
            
            self.send_request(&request, None);
            self.recv_reply(&mut reply, None);
            
            match reply.apc_type {
                ApcType::ApcNone => {}
                _ => break reply.status
            }
        }
    }
    
    fn send_request<T>(&mut self, request: &T, args: Option<Vec<&[u8]>>) {
        let mut req = [0u8;64];
        let req_bytes = unsafe {&*slice_from_raw_parts(&*(request as *const T as *const u8), size_of::<T>())};
        for (src, dst) in req_bytes.iter().zip(req.iter_mut()) {
            *dst = *src;
        }
        
        let req_id = u32::from_ne_bytes(req[0..4].try_into().unwrap());
        let req_id_bytes = (req_id - self.request_offset_table[req_id as usize]).to_ne_bytes(); //subtract to accommodate for different wineserver protocol versions
        for (src, dst) in req_id_bytes.iter().zip(req.iter_mut()) {
            *dst = *src;
        }
        
        if let Some(args) = args {
            let iov = {
                let mut vec: Vec<IoSlice> = Vec::with_capacity(args.len()+1);
                vec.push(IoSlice::new(&req));
                for a in args {
                    vec.push(IoSlice::new(a));
                }
                vec
            };
            self.request_pipe.write_vectored(&iov).unwrap();
        } else {
            self.request_pipe.write_all(&req).unwrap();
        }
    }
    
    fn recv_reply<T>(&mut self, reply: &mut T, args: Option<&mut Vec<u8>>) {
        let mut reply_buf = [0u8;64];
        let iov = &mut vec![
            IoSliceMut::new(unsafe {slice::from_raw_parts_mut(reply as *mut T as *mut u8, size_of_val(reply))}),
            IoSliceMut::new(&mut reply_buf[size_of_val(reply)..64])
        ];
        self.reply_pipe_r.read_vectored(iov).unwrap();
        let reply_extra = u32::from_le_bytes(
            unsafe {
                slice::from_raw_parts((reply as *const T as *const u8).add(4), 4)
            }.try_into().unwrap()
        );
        
        if reply_extra != 0 {
            if let Some(mut args) = args {
                args.resize(reply_extra as usize, 0);
                self.reply_pipe_r.read(&mut args).unwrap();
            } else {
                panic!("Reply with extra data recieved while args is None");
            }
        }
    }
    
    fn recv_wait<T>(&mut self, reply: &mut T) {
        let iov = &mut vec![
            IoSliceMut::new(unsafe {slice::from_raw_parts_mut(reply as *mut T as *mut u8, size_of_val(reply))}),
        ];
        self.wait_pipe_r.read_vectored(iov).unwrap();
    }
    
    pub fn get_fusion_handle_preinit(request_pipe: &mut File, reply_pipe: &mut PipeReader, pid: i32, request_offset_table: &[u32]) -> Option<u32> {
        let mut request = ListProcessesRequest {
            header: RequestHeader { request: Request::ReqListProcesses, request_size: 0, reply_size: 0 }
        };
        let mut reply = unsafe {mem::zeroed::<ListProcessesReply>()};
        
        Self::send_request_preinit(request_pipe, &request, None, request_offset_table);
        Self::recv_reply_preinit(reply_pipe, &mut reply, None);
        request.header.reply_size = reply.info_size;
        Self::send_request_preinit(request_pipe, &request, None, request_offset_table);
        let mut reply_data: Vec<u8> = Vec::new();
        Self::recv_reply_preinit(reply_pipe, &mut reply, Some(&mut reply_data));
        
        let mut data_off = 0;
        let mut windows_pid: Option<u32> = None;
        for _process_idx in 0 .. reply.process_count {
            data_off = (data_off + 7) & !7;
            let process_info = unsafe {
                &*(&reply_data[data_off] as *const u8 as *const ProcessInfo)
            };
            data_off += size_of::<ProcessInfo>();
            
            if process_info.unix_pid == pid {
                //let name = String::from_utf16_lossy(
                //    unsafe {
                //        &*(
                //            &reply_data[data_off .. data_off + (process_info.name_len >> 1) as usize] as *const [u8] as *const [u16]
                //        )
                //    }
                //);
                windows_pid = Some(process_info.pid);
                break;
            }
            data_off = (data_off + process_info.name_len as usize + 7) & !7;
            data_off += size_of::<ThreadInfo>() * process_info.thread_count as usize;
        }
        
        if let Some(windows_pid) = windows_pid {
            let request = OpenProcessRequest {
                header: RequestHeader { request: Request::ReqOpenProcess, request_size: 0, reply_size: 0 },
                pid:    windows_pid,
                access: 0x8, //PROCESS_VM_OPERATION
                attrs:  0,
            };
            let mut reply = unsafe {mem::zeroed::<OpenProcessReply>()};
            
            Self::send_request_preinit(request_pipe, &request, None, request_offset_table);
            Self::recv_reply_preinit(reply_pipe, &mut reply, None);
            
            return Some(reply.handle);
        }
        
        None
    }
    
    pub fn init_first_thread(request_pipe: &mut File, reply_pipe: &mut PipeReader, reply_pipe_w: &PipeWriter, wait_pipe_w: &PipeWriter, request_offset_table: &[u32]) {
        let request = InitFirstThreadRequest {
            header: RequestHeader { request: Request::ReqInitFirstThread, request_size: 0, reply_size: 0 },
            unix_pid: process::id() as i32,
            unix_tid: gettid() as i32,
            debug_level: 0,
            reply_fd: reply_pipe_w.as_raw_fd(),
            wait_fd: wait_pipe_w.as_raw_fd(),
        };
        let mut reply = unsafe {mem::zeroed::<InitFirstThreadReply>()};
        Self::send_request_preinit(request_pipe, &request, None, request_offset_table);
        Self::recv_reply_preinit(reply_pipe, &mut reply, None);
    }
    
    fn recv_reply_preinit<T>(reply_pipe: &mut PipeReader, reply: &mut T, args: Option<&mut Vec<u8>>) {
        let mut reply_buf = [0u8;64];
        let iov = &mut vec![
            IoSliceMut::new(unsafe {slice::from_raw_parts_mut(reply as *mut T as *mut u8, size_of_val(reply))}),
            IoSliceMut::new(&mut reply_buf[size_of_val(reply)..64])
        ];
        reply_pipe.read_vectored(iov).unwrap();
        let reply_extra = u32::from_le_bytes(
            unsafe {
                slice::from_raw_parts((reply as *const T as *const u8).add(4), 4)
            }.try_into().unwrap()
        );
        
        if reply_extra != 0 {
            if let Some(mut args) = args {
                args.resize(reply_extra as usize, 0);
                reply_pipe.read(&mut args).unwrap();
            } else {
                panic!("Reply with extra data recieved while args is None");
            }
        }
    }
    
    fn send_request_preinit<T>(request_pipe: &mut File, request: &T, args: Option<Vec<&[u8]>>, request_offset_table: &[u32]) {
        let mut req = [0u8;64];
        let req_bytes = unsafe {&*slice_from_raw_parts(&*(request as *const T as *const u8), size_of::<T>())};
        for (src, dst) in req_bytes.iter().zip(req.iter_mut()) {
            *dst = *src;
        }
        
        let req_id = u32::from_ne_bytes(req[0..4].try_into().unwrap());
        let req_id_bytes = (req_id - request_offset_table[req_id as usize]).to_ne_bytes(); //subtract to accommodate for different wineserver protocol versions
        for (src, dst) in req_id_bytes.iter().zip(req.iter_mut()) {
            *dst = *src;
        }
        
        if let Some(args) = args {
            let iov = {
                let mut vec: Vec<IoSlice> = Vec::with_capacity(args.len()+1);
                vec.push(IoSlice::new(&req));
                for a in args {
                    vec.push(IoSlice::new(a));
                }
                vec
            };
            request_pipe.write_vectored(&iov).unwrap();
        } else {
            request_pipe.write_all(&req).unwrap();
        }
    }
    
    pub fn send_fd_preinit(wineserver_socket: &mut UnixStream, fd: i32) {
        let send_fd_struct = SendFd {
            fd,
            tid: 0,
        };
        
        let req_bytes = unsafe {&*slice_from_raw_parts(&*(&send_fd_struct as *const SendFd as *const u8), size_of::<SendFd>())};
        
        let mut ancillary_buf = [0;256];
        let mut ancillary = SocketAncillary::new(&mut ancillary_buf);
        ancillary.add_fds(&[fd]);
        
        wineserver_socket.send_vectored_with_ancillary(
            &[IoSlice::new(&req_bytes)],
            &mut ancillary
        ).unwrap();
    }
}
