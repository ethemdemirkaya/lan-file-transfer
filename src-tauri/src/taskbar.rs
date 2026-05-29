//! Drive the Windows taskbar progress overlay (the green bar that appears
//! on top of the app's taskbar button while a download is in flight).
//! Uses `ITaskbarList3` via the `windows` crate.

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::{Mutex, OnceLock};

    use windows::core::Interface;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        ITaskbarList3, TaskbarList, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL,
        TBPF_PAUSED, TBPFLAG,
    };

    // ITaskbarList3 is COM and the COM apartment must be initialised on
    // every thread that touches it. We stash a single instance behind a
    // OnceLock; all calls go through a Mutex so we don't race the COM
    // singleton across spawned tasks.
    struct TaskbarHandle {
        list: ITaskbarList3,
    }
    // SAFETY: ITaskbarList3 calls are serialised through the Mutex below.
    unsafe impl Send for TaskbarHandle {}

    static TASKBAR: OnceLock<Mutex<Option<TaskbarHandle>>> = OnceLock::new();

    fn init_once() -> &'static Mutex<Option<TaskbarHandle>> {
        TASKBAR.get_or_init(|| {
            let res = unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
                CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            };
            Mutex::new(res.ok().map(|list| TaskbarHandle { list }))
        })
    }

    pub enum State {
        None,
        Normal,
        Paused,
        Indeterminate,
    }

    impl State {
        fn flag(&self) -> TBPFLAG {
            match self {
                State::None => TBPF_NOPROGRESS,
                State::Normal => TBPF_NORMAL,
                State::Paused => TBPF_PAUSED,
                State::Indeterminate => TBPF_INDETERMINATE,
            }
        }
    }

    pub fn set(hwnd: isize, value: u64, total: u64, state: State) {
        let lock = init_once().lock().unwrap();
        let Some(h) = lock.as_ref() else { return };
        let hwnd = HWND(hwnd as *mut _);
        unsafe {
            let _ = h.list.SetProgressState(hwnd, state.flag());
            if matches!(state, State::Normal | State::Paused) {
                let _ = h.list.SetProgressValue(hwnd, value, total.max(1));
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use imp::{set as set_progress, State as ProgressState};

#[cfg(not(target_os = "windows"))]
pub enum ProgressState {
    None,
    Normal,
    Paused,
    Indeterminate,
}

#[cfg(not(target_os = "windows"))]
pub fn set_progress(_hwnd: isize, _value: u64, _total: u64, _state: ProgressState) {}
