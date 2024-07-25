use std::ffi::OsString;

/// The default target to pass to cargo, to workaround issue #11.
pub fn default_target() -> &'static str {
    current_platform::CURRENT_PLATFORM
}

/// Gets the path to the asan DLL required for the asan instrumented binary to run.
#[cfg(target_env = "msvc")]
pub fn get_asan_path() -> Option<std::path::PathBuf> {
    // The asan DLL sits next to cl & link.exe. So grab the parent path.
    Some(
        cc::windows_registry::find_tool(default_target(), "link.exe")?
            .path()
            .parent()?
            .to_owned(),
    )
}

/// Append a value to the PATH variable
#[cfg(target_env = "msvc")]
pub fn append_to_pathvar(path: &std::path::Path) -> Option<OsString> {
    use std::env;

    if let Some(current) = env::var_os("PATH") {
        let mut current = env::split_paths(&current).collect::<Vec<_>>();
        current.push(path.to_path_buf());
        return env::join_paths(current).ok();
    }

    return None;
}

/// Add current process to a Windows Job Object
/// This means that when the current process is terminated, all children are as well.
#[cfg(target_env = "msvc")]
pub fn create_job_object() -> anyhow::Result<()> {
    use anyhow::bail;
    use std::{mem::MaybeUninit, ptr};
    use windows_sys::Win32::System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::GetCurrentProcess,
    };

    // Safety: Both parameters are optional. The first parameter being null
    // ensures that the child processes cannot inherit the job handle.
    // The second argument means that it is anonymous.
    let maybe_handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };

    // If the handle is null, the above function failed. If it is non-null, it succeeded.
    if maybe_handle == 0 {
        bail!("invalid job handle returned");
    }

    let mut info = MaybeUninit::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>::uninit();

    // Safety:
    // We pass in a MaybeUninit extended info object, and then also give the size of it as the size parameter.
    // The return length is optional, so we set it to null.
    let err = unsafe {
        QueryInformationJobObject(
            maybe_handle,
            JobObjectExtendedLimitInformation,
            std::ptr::from_mut(&mut info) as _,
            std::mem::size_of_val(&info) as _,
            ptr::null_mut(),
        )
    };

    // This function returns zero on failure.
    if err == 0 {
        bail!("JobObject information query failed");
    }

    // Safety:
    // The query infomation called suceeded, so it is now init.
    let mut info = unsafe { info.assume_init() };

    // Flag for killing children upon closure.
    info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    // Safety:
    // Handle is valid & has set attributes rights
    // Info object is not exclusively held anywhere and it a valid object for this API.
    let err = unsafe {
        SetInformationJobObject(
            maybe_handle,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&info) as _,
            std::mem::size_of_val(&info) as _,
        )
    };

    // Failure is zero
    if err == 0 {
        bail!("setting job object information failed");
    }

    // Safety:
    // I do not see any preconditions on this function.
    // Receive the current processes's handle.
    let this_process = unsafe { GetCurrentProcess() };

    // Safety:
    // The job is valid and has the ASSIGN_PROCESS access right because we created it.
    // the handle has the terminate & set quote access right because it is our handle.
    let err = unsafe { AssignProcessToJobObject(maybe_handle, this_process) };

    if err == 0 {
        bail!("failed to assign current process to job object")
    }

    // Note:
    // We could return the handle, but there is no reason to. All child processes will automatically
    // be added to the job. As soon as the handle we created is closed, all children will be terminated.
    //
    // Since we added ourselves to the Job, is we close the handle manually we will terminate ourselves.
    // If we allowed our process to exit by exiting main(), Windows will close the handle for us, terminating all children.
    Ok(())
}
