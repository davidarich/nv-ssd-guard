use winapi::shared::minwindef::FALSE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::processthreadsapi::OpenThread;
use winapi::um::processthreadsapi::ResumeThread;
use winapi::um::processthreadsapi::SuspendThread;
use winapi::um::processthreadsapi::TerminateProcess;
use winapi::um::tlhelp32::CreateToolhelp32Snapshot;
use winapi::um::tlhelp32::PROCESSENTRY32W;
use winapi::um::tlhelp32::Process32FirstW;
use winapi::um::tlhelp32::Process32NextW;
use winapi::um::tlhelp32::TH32CS_SNAPPROCESS;
use winapi::um::tlhelp32::TH32CS_SNAPTHREAD;
use winapi::um::tlhelp32::THREADENTRY32;
use winapi::um::tlhelp32::Thread32First;
use winapi::um::tlhelp32::Thread32Next;
use winapi::um::winnt::HANDLE;
use winapi::um::winnt::PROCESS_TERMINATE;
use winapi::um::winnt::THREAD_SUSPEND_RESUME;

pub fn find_process_id_by_name(name: &str) -> u32 {
    unsafe {
        let pe: &mut PROCESSENTRY32W = &mut std::mem::zeroed();
        (*pe).dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

        if Process32FirstW(snapshot, pe) == 1 {
            loop {
                let proc_name = String::from_utf16_lossy(&pe.szExeFile);
                let proc_name = proc_name.trim_end_matches(char::from(0));

                if proc_name == name {
                    break;
                }

                if Process32NextW(snapshot, pe) == 0 {
                    (*pe).th32ProcessID = 0;
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        return (*pe).th32ProcessID;
    }
}

pub fn kill_processes_by_name(name: &str) -> bool {
    let mut success = true;
    loop {
        let pid = find_process_id_by_name(name);
        if pid == 0 {
            break;
        }

        let process: HANDLE = unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, pid) };
        if process.is_null() {
            success = false;
            break;
        }

        if unsafe { TerminateProcess(process, 0xFF) } == 0 {
            unsafe {
                CloseHandle(process);
            }
            success = false;
            break;
        }

        unsafe {
            CloseHandle(process);
        }
    }

    return success;
}

pub fn suspend_process(pid: u32) -> (u32, bool) {
    unsafe {
        let mut has_err = false;
        let mut count: u32 = 0;

        let te: &mut THREADENTRY32 = &mut std::mem::zeroed();
        (*te).dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

        let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);

        if Thread32First(snapshot, te) == 1 {
            loop {
                if pid == (*te).th32OwnerProcessID {
                    let tid = (*te).th32ThreadID;

                    let thread: HANDLE = OpenThread(THREAD_SUSPEND_RESUME, FALSE, tid);
                    has_err |= SuspendThread(thread) as i32 == -1i32;

                    CloseHandle(thread);
                    count += 1;
                }

                if Thread32Next(snapshot, te) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        return (count, has_err);
    }
}

pub fn resume_process(pid: u32) -> (u32, bool) {
    unsafe {
        let mut has_err = false;
        let mut count: u32 = 0;

        let te: &mut THREADENTRY32 = &mut std::mem::zeroed();
        (*te).dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

        let snapshot: HANDLE = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);

        if Thread32First(snapshot, te) == 1 {
            loop {
                if pid == (*te).th32OwnerProcessID {
                    let tid = (*te).th32ThreadID;

                    let thread: HANDLE = OpenThread(THREAD_SUSPEND_RESUME, FALSE, tid);
                    has_err |= ResumeThread(thread) as i32 == -1i32;

                    CloseHandle(thread);
                    count += 1;
                }

                if Thread32Next(snapshot, te) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        return (count, has_err);
    }
}
