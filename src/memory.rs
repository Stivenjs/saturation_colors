//! Utilidades de Windows para liberar páginas físicas inactivas del proceso.

/// Solicita al sistema operativo que pode páginas no utilizadas del proceso actual.
///
/// Solicita a Windows que pode páginas no utilizadas del *working set*.
pub fn trim_working_set() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

        let result = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        if result == 0 {
            tracing::debug!("Windows no pudo podar el working set del proceso");
        } else {
            tracing::debug!("working set del proceso podado");
        }
    }
}
