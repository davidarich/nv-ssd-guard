use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::TaskScheduler::{
    ITaskFolder, ITaskService, TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN, TaskScheduler,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{BSTR, HRESULT, IUnknown};

/// HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND), returned when a task does not exist.
const HRESULT_FILE_NOT_FOUND: HRESULT = HRESULT(0x8007_0002_u32 as i32);
/// E_ACCESSDENIED, typically returned when the caller is not elevated.
const HRESULT_ACCESS_DENIED: HRESULT = HRESULT(0x8007_0005_u32 as i32);

/// Creates (or updates) a Windows Task Scheduler task, named `name`, that runs
/// the currently running executable on user login.
///
/// Registers the task definition XML directly through the Task Scheduler COM
/// API (`ITaskService` / `ITaskFolder::RegisterTask`), so no temporary file or
/// `schtasks.exe` is involved and the XML text encoding cannot mismatch.
/// Returns `(created, error)` where `created` is `true` if the task was
/// registered and `error` is the error message (empty on success).
pub fn create_scheduled_task(name: &str) -> (bool, String) {
    // Resolve the path of the currently running executable.
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return (false, format!("failed to get current exe path: {}", e));
        }
    };

    let xml = task_xml(&exe.to_string_lossy());
    let name = name.to_owned();

    let result = run_on_com_thread(move || {
        let folder = root_folder().map_err(|e| describe("failed to connect to Task Scheduler", &e))?;
        let empty = VARIANT::default();
        unsafe {
            folder.RegisterTask(
                &BSTR::from(name.as_str()),
                &BSTR::from(xml.as_str()),
                TASK_CREATE_OR_UPDATE.0,
                &empty,
                &empty,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty,
            )
        }
        .map(|_| ())
        .map_err(|e| describe("failed to register scheduled task", &e))
    });

    finish(result)
}

/// Deletes the Windows Task Scheduler task named `name`.
///
/// Returns `(deleted, error)` where `deleted` is `true` if the task was removed
/// or did not exist, and `error` is the error message (empty on success).
pub fn delete_scheduled_task(name: &str) -> (bool, String) {
    let name = name.to_owned();

    let result = run_on_com_thread(move || {
        let folder = root_folder().map_err(|e| describe("failed to connect to Task Scheduler", &e))?;
        match unsafe { folder.DeleteTask(&BSTR::from(name.as_str()), 0) } {
            Ok(()) => Ok(()),
            // Nothing to delete is not a failure.
            Err(e) if e.code() == HRESULT_FILE_NOT_FOUND => Ok(()),
            Err(e) => Err(describe("failed to delete scheduled task", &e)),
        }
    });

    finish(result)
}

/// Builds the task definition XML for a logon-triggered task running `command`.
fn task_xml(command: &str) -> String {
    // Escape XML-special characters in the command.
    let escaped_command = command
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");

    // No XML declaration: the text is handed to the API as a UTF-16 BSTR, so
    // there is no file encoding to declare (or to get wrong).
    format!(
        r#"<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
    </Exec>
  </Actions>
</Task>"#,
        escaped_command
    )
}

/// Connects to the local Task Scheduler service and returns its root folder.
/// Must be called on a thread with COM initialized.
fn root_folder() -> windows::core::Result<ITaskFolder> {
    let service: ITaskService =
        unsafe { CoCreateInstance(&TaskScheduler, None::<&IUnknown>, CLSCTX_INPROC_SERVER)? };
    // Empty VARIANTs: local machine, current user.
    let empty = VARIANT::default();
    unsafe { service.Connect(&empty, &empty, &empty, &empty)? };
    unsafe { service.GetFolder(&BSTR::from("\\")) }
}

/// Runs `f` on a fresh thread with COM initialized as multithreaded.
///
/// The UI thread may already have COM initialized in a different apartment
/// mode (e.g. by the windowing library), so a dedicated thread avoids
/// `RPC_E_CHANGED_MODE`. All COM objects are created and dropped inside `f`,
/// before COM is uninitialized.
fn run_on_com_thread<F>(f: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let handle = std::thread::spawn(move || {
        if let Err(e) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok() {
            return Err(describe("failed to initialize COM", &e));
        }
        let result = f();
        unsafe { CoUninitialize() };
        result
    });

    handle
        .join()
        .unwrap_or_else(|_| Err("task scheduler thread panicked".to_string()))
}

/// Formats a COM error with context, adding a hint for access-denied errors.
fn describe(context: &str, e: &windows::core::Error) -> String {
    let hint = if e.code() == HRESULT_ACCESS_DENIED {
        " (run nv-ssd-guard as administrator to create a task with highest privileges)"
    } else {
        ""
    };
    format!("{}: {}{}", context, e, hint)
}

/// Converts a result into the `(success, error)` tuple returned by this module.
fn finish(result: Result<(), String>) -> (bool, String) {
    match result {
        Ok(()) => (true, String::new()),
        Err(msg) => {
            eprintln!("{}", msg);
            (false, msg)
        }
    }
}
