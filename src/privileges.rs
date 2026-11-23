use std::mem::size_of;
use std::ptr::null_mut;
use winapi::shared::ntdef::LUID;
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
use winapi::um::securitybaseapi::AdjustTokenPrivileges;
use winapi::um::winbase::LookupPrivilegeValueA;
use winapi::um::winnt::{
    HANDLE, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    TOKEN_QUERY,
};

pub fn acquire_debug_privilege() {
    unsafe {
        let mut token_handle: HANDLE = 0 as _;
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token_handle,
        );
        let privileges = LUID_AND_ATTRIBUTES {
            Attributes: SE_PRIVILEGE_ENABLED,
            Luid: LUID {
                LowPart: 0,
                HighPart: 0,
            },
        };
        let mut token_privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [privileges; 1],
        };
        let se_debug_privilege = "SeDebugPrivilege\0";
        let _lookup_privelege_result = LookupPrivilegeValueA(
            null_mut(),
            se_debug_privilege.as_ptr() as *const i8,
            &mut token_privileges.Privileges[0].Luid,
        );
        let _adjust_token_privileges_result = AdjustTokenPrivileges(
            token_handle,
            0,
            &mut token_privileges,
            size_of::<TOKEN_PRIVILEGES>() as _,
            null_mut(),
            null_mut(),
        );
    }
}

/// Returns true if the current process is running with an elevated
/// (administrator) token.
pub fn is_elevated() -> bool {
    use winapi::shared::minwindef::DWORD;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TOKEN_ELEVATION, TokenElevation};

    unsafe {
        let mut token_handle: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length: DWORD = 0;
        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as *mut _,
            size_of::<TOKEN_ELEVATION>() as DWORD,
            &mut return_length,
        );
        CloseHandle(token_handle);

        result != 0 && elevation.TokenIsElevated != 0
    }
}
