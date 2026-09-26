//! Ties ffmpeg processes to the app's lifetime: when the app exits (or crashes),
//! Windows closes the job handle and kills every process assigned to it.

use std::process::Child;

#[cfg(windows)]
pub struct ProcessJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
// SAFETY: a job object handle can be used from any thread.
unsafe impl Send for ProcessJob {}
#[cfg(windows)]
unsafe impl Sync for ProcessJob {}

#[cfg(windows)]
impl ProcessJob {
    pub fn new() -> std::io::Result<Self> {
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: plain Win32 calls with owned, zero-initialised structs.
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                let e = std::io::Error::last_os_error();
                windows_sys::Win32::Foundation::CloseHandle(h);
                return Err(e);
            }
            Ok(Self(h))
        }
    }

    pub fn assign(&self, child: &Child) -> std::io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: both handles are valid for the duration of the call.
        let ok = unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(self.0, child.as_raw_handle() as _)
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        // SAFETY: we own the handle.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(not(windows))]
pub struct ProcessJob;

#[cfg(not(windows))]
impl ProcessJob {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self)
    }
    pub fn assign(&self, _child: &Child) -> std::io::Result<()> {
        Ok(())
    }
}

/// The app-wide job every ffmpeg is assigned to. Never dropped, so it lives until exit.
pub fn app_job() -> Option<&'static ProcessJob> {
    static JOB: std::sync::OnceLock<Option<ProcessJob>> = std::sync::OnceLock::new();
    JOB.get_or_init(|| ProcessJob::new().ok()).as_ref()
}
