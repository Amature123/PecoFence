//! Process helpers: spawn a detached helper, wait for a process to exit.

use crate::bindings::*;
use crate::wide::to_wide;
use windows_core::{Error, PCWSTR, PWSTR, Result};

const SYNCHRONIZE: u32 = 0x0010_0000;

/// Blocks until the process with `pid` exits. Returns immediately if it cannot be opened.
pub fn wait_for_process_exit(pid: u32) {
    // SAFETY: plain FFI calls; the handle is closed afterwards.
    unsafe {
        let h = OpenProcess(SYNCHRONIZE, false, pid);
        if h.0.is_null() {
            return;
        }
        let _ = WaitForSingleObject(h, INFINITE);
        let _ = CloseHandle(h);
    }
}

pub fn current_pid() -> u32 {
    // SAFETY: plain FFI call.
    unsafe { GetCurrentProcessId() }
}

/// True when running from an MSIX package (Microsoft Store install). Packaged processes
/// get virtualized HKCU writes, so the Run-key autostart is replaced by the manifest's
/// `windows.startupTask`, which users control under Settings > Apps > Startup.
pub fn is_packaged() -> bool {
    const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;
    windows_core::link!("kernel32.dll" "system" fn GetCurrentPackageFullName(length: *mut u32, name: windows_core::PWSTR) -> i32);
    let mut len = 0u32;
    // SAFETY: querying the required length only; a null buffer is documented as valid.
    let status = unsafe { GetCurrentPackageFullName(&mut len, PWSTR::null()) };
    status != APPMODEL_ERROR_NO_PACKAGE
}

/// Asks Windows to start this program again (without arguments) after it closed for an update:
/// a Store package update or an installer using Restart Manager. Not after a crash, a hang or
/// a reboot, where the watchdog and the startup entry take care of it.
pub fn register_restart_after_update() -> Result<()> {
    const RESTART_NO_CRASH: u32 = 1;
    const RESTART_NO_HANG: u32 = 2;
    const RESTART_NO_REBOOT: u32 = 8;
    windows_core::link!("kernel32.dll" "system" fn RegisterApplicationRestart(pwzcommandline: PCWSTR, dwflags: u32) -> windows_core::HRESULT);
    // SAFETY: a null command line is documented as valid (restart without arguments).
    unsafe {
        RegisterApplicationRestart(
            PCWSTR::null(),
            RESTART_NO_CRASH | RESTART_NO_HANG | RESTART_NO_REBOOT,
        )
    }
    .ok()
}

/// Starts `exe` with `args` as a detached process (no console window). Returns its pid.
pub fn spawn_detached(exe: &str, args: &[&str]) -> Result<u32> {
    let mut cmd = format!("\"{exe}\"");
    for a in args {
        cmd.push(' ');
        cmd.push('"');
        cmd.push_str(a);
        cmd.push('"');
    }
    let mut cmd_w = to_wide(&cmd);
    let exe_w = to_wide(exe);
    let si = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut pi = PROCESS_INFORMATION::default();
    // SAFETY: buffers outlive the call; handles are closed afterwards.
    unsafe {
        let ok = CreateProcessW(
            PCWSTR(exe_w.as_ptr()),
            Some(PWSTR(cmd_w.as_mut_ptr())),
            None,
            None,
            false,
            CREATE_NO_WINDOW as u32,
            None,
            PCWSTR::null(),
            &si,
            &mut pi,
        );
        if !ok.as_bool() {
            return Err(Error::from_thread());
        }
        let _ = CloseHandle(pi.hThread);
        let _ = CloseHandle(pi.hProcess);
        Ok(pi.dwProcessId)
    }
}
