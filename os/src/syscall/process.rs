//! Process management syscalls
use crate::{
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,
    },
};
use crate::config::PAGE_SIZE;
use crate::mm::{get_page_from_vir, MapPermission, VirtAddr, VirtPageNum};
use crate::task::{get_syscall_times, TaskInfo, TaskStatus, TASK_MANAGER};
use crate::timer::{ get_time_ms, get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}


/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let va = VirtAddr(ts as usize);
    if let Some(pa) = get_page_from_vir(va) {
        let time_us = get_time_us();
        let tv = TimeVal {
            sec: time_us / 1_000_000,
            usec: time_us % 1_000_000,
        };
        let ts = pa.0 as *mut TimeVal;
        unsafe {
            *ts = tv;
        }
        0
    } else {
        -1
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let va = VirtAddr(_ti as usize);
    if let Some(pa) = get_page_from_vir(va) {
        let task_info = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: get_syscall_times(),
            time: get_time_ms(),
        };
        let ti = pa.0 as *mut TaskInfo;
        unsafe {
            *ti = task_info;
        }
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, _len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap TEST");
    // let mut inner = TASK_MANAGER.inner.exclusive_access();
    // let current = inner.current_task;
    // inner.tasks[current].sys_call[6] += 1;
    if start & (PAGE_SIZE - 1) != 0{
        return -1;
    }else if port & !0x7 != 0 || port & 0x7 == 0{
        return -1;
    }
    let permission= MapPermission::from_bits((port as u8) << 1).unwrap() | MapPermission::U;
    let mut inner =  TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let memory_set = &mut inner.tasks[current].memory_set;

    //check the pre vpn
    let start_vpn = VirtPageNum::from(VirtAddr(start).floor());
    let end_vpn = VirtPageNum::from(VirtAddr(start + _len).ceil());
    for vpn in start_vpn.0 .. end_vpn.0 {
        if let Some(vpn) = memory_set.translate(VirtPageNum(vpn)) {
            if vpn.is_valid() {
                println!("mmap failed: address already mapped");
                return -1;
            }
        }
    }
    //map new space
    memory_set.insert_framed_area(VirtAddr::from(start) , VirtAddr::from(start+_len),permission);
    drop(inner);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap TEST");
    let mut inner =  TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let  memory_set = &mut inner.tasks[current].memory_set;
    if start & (PAGE_SIZE-1) != 0 {
        return -1;
    }
    let start_vpn = VirtPageNum::from(VirtAddr(start).floor());
    let end_vpn = VirtPageNum::from(VirtAddr(start + _len).ceil());
    for vpn in start_vpn.0 .. end_vpn.0 {
        if let Some(vpn) = memory_set.translate(VirtPageNum(vpn)) {
            if !vpn.is_valid() {
                println!("mmap failed: address already mapped");
                return -1;
            }
        }
    }
    for area in memory_set.areas.iter_mut() {
        if area.vpn_range.get_start() == VirtAddr::from(start).floor() && area.vpn_range.get_end() == VirtAddr::from(start+_len).ceil(){
            area.unmap(&mut memory_set.page_table);
        }
    }
    drop(inner);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
