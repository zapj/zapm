use anyhow::Result;

#[cfg(target_os = "windows")]
extern crate winapi;
use winapi::um::processthreadsapi::{CreateProcessA, PROCESS_INFORMATION, STARTUPINFOA};
use winapi::um::winbase::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
use winapi::um::winnt::PROCESS_TERMINATE;
use winapi::shared::minwindef::DWORD;

#[cfg(target_os = "linux")]
extern crate libc;

/// 启动守护进程（跨平台实现）
pub fn start_daemon() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Windows 平台实现

        use std::fs;

        use crate::config;
        let mut si: STARTUPINFOA = unsafe { std::mem::zeroed() };
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        
        si.cb = std::mem::size_of::<STARTUPINFOA>() as u32;
        
        let command = std::ffi::CString::new("zapm server").unwrap();
        
        unsafe {
            if CreateProcessA(
                std::ptr::null(),
                command.as_ptr() as *mut i8,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS,
                std::ptr::null_mut(),
                std::ptr::null(),
                &mut si,
                &mut pi,
            ) == 0
            {
                return Err(anyhow::anyhow!("Failed to create daemon process"));
            }
        }
        let _ = fs::write(config::CONFIG_PATH.join("zapm.pid").as_path(), format!("{}", pi.dwProcessId));
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux 平台实现
        match unsafe { libc::fork() } {
            -1 => return Err(anyhow::anyhow!("Failed to fork daemon process")),
            0 => {
                // 子进程
                unsafe { libc::setsid(); } // 创建新会话
                
                // 重定向标准输入输出到 /dev/null
                let null = std::fs::File::open("/dev/null").unwrap();
                let _ = std::os::unix::io::IntoRawFd::into_raw_fd(null);
                
                // 执行守护进程逻辑
                let _ = process::Command::new("zapm")
                    .arg("daemon")
                    .spawn()
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
            _ => return Ok(()), // 父进程直接退出
        }
    }
    
    Ok(())
}

/// 守护进程主循环
pub fn stop_daemon() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::fs;

        use crate::config;
        fs::read_to_string(config::CONFIG_PATH.join("zapm.pid").as_path())?.parse::<u32>().map(|pid| {
            let _ = terminate_process_by_pid(pid);
            let _ = fs::remove_file(config::CONFIG_PATH.join("zapm.pid").as_path());
        })?;
    }
    Ok(())
}


fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
 
    #[cfg(target_os = "windows")]
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid as DWORD);
        if handle.is_null() {
            return Err(format!("Failed to open process with PID {}. Error: {}", pid, std::io::Error::last_os_error()));
        }

        if TerminateProcess(handle, 0) == 0 { // 0 indicates failure
            return Err(format!("Failed to terminate process with PID {}. Error: {}", pid, std::io::Error::last_os_error()));
        }

        // CloseHandle(handle); // Not strictly necessary after TerminateProcess, but good practice
    }

    #[cfg(target_os = "linux")]
    {
        use std::io;
        use std::io::Error;
        
        unsafe {
            if libc::kill(pid, libc::SIGKILL) != 0 {
                return Err(Error::last_os_error());
            }
        }
    }
    Ok(())
}

