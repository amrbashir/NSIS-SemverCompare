use std::mem::size_of;

use pluginapi::{decode_wide, exdll_init, popstring, pushint, stack_t, wchar_t};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    },
};

/// Test if there is a running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn FindProcess(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    if !get_processes(&name).is_empty() {
        pushint(0);
    } else {
        pushint(1);
    }
}

/// Kill all running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn KillProcess(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    let processes = get_processes(&name);

    if !processes.is_empty() && processes.into_iter().map(kill).all(|b| b) {
        pushint(0);
    } else {
        pushint(1);
    }
}

fn kill(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        let success = TerminateProcess(handle, 1);
        CloseHandle(handle);
        success != 0
    }
}

fn get_processes(name: &str) -> Vec<u32> {
    let current_pid = std::process::id();
    let mut processes = Vec::new();

    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

        let mut process = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };

        if Process32FirstW(handle, &mut process) != 0 {
            while Process32NextW(handle, &mut process) != 0 {
                if current_pid != process.th32ProcessID
                    && decode_wide(&process.szExeFile)
                        .to_str()
                        .unwrap_or_default()
                        .to_lowercase()
                        == name.to_lowercase()
                {
                    processes.push(process.th32ProcessID);
                }
            }
        }

        CloseHandle(handle);
    }

    processes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_process() {
        let processes = get_processes("explorer.exe");
        dbg!(&processes);
        assert!(!processes.is_empty());
    }

    #[test]
    fn kill_process() {
        let processes = get_processes("something_that_doesnt_exist.exe");
        dbg!(&processes);
        // TODO: maybe find some way to spawn a dummy process we can kill here?
        // This will return true on empty iterators so it's basically no-op right now
        assert!(processes.into_iter().map(kill).all(|b| b));
    }
}
