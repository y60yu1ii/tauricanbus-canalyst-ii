use libloading::Library;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tauri::State;
use serde::Serialize;

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciCanObj {
    pub id: u32,
    pub time_stamp: u32,
    pub time_flag: u8,
    pub send_type: u8,
    pub remote_flag: u8,
    pub extern_flag: u8,
    pub data_len: u8,
    pub data: [u8; 8],
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciInitConfig {
    pub acc_code: u32,
    pub acc_mask: u32,
    pub reserved: u32,
    pub filter: u8,
    pub timing0: u8,
    pub timing1: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct VciBoardInfo {
    pub hw_version: u16,
    pub fw_version: u16,
    pub dr_version: u16,
    pub in_version: u16,
    pub irq_num: u16,
    pub can_num: u8,
    pub str_serial_num: [u8; 20],
    pub str_hw_type: [u8; 40],
    pub reserved: [u16; 4],
}

impl Default for VciBoardInfo {
    fn default() -> Self {
        Self {
            hw_version: 0,
            fw_version: 0,
            dr_version: 0,
            in_version: 0,
            irq_num: 0,
            can_num: 0,
            str_serial_num: [0; 20], 
            str_hw_type: [0; 40],    
            reserved: [0; 4],
        }
    }
}


#[derive(Serialize)]
pub struct DeviceInfo {
    pub index: i32,
    pub serial_number: String,
    pub firmware_version: u16,
}

pub struct CanLibrary {
    _lib: Arc<Library>,
    pub vci_open_device: unsafe extern "stdcall" fn(u32, u32, u32) -> i32,
    pub vci_close_device: unsafe extern "stdcall" fn(u32, u32) -> i32,
    pub vci_init_can: unsafe extern "stdcall" fn(u32, u32, u32, *const VciInitConfig) -> i32,
    pub vci_start_can: unsafe extern "stdcall" fn(u32, u32, u32) -> i32,
    pub vci_transmit: unsafe extern "stdcall" fn(u32, u32, u32, *const VciCanObj, u32) -> i32,
    pub vci_receive: unsafe extern "stdcall" fn(u32, u32, u32, *mut VciCanObj, u32, i32) -> i32,
    pub vci_find_usb_device2: unsafe extern "stdcall" fn(*mut VciBoardInfo) -> i32,
    pub vci_read_board_info: unsafe extern "stdcall" fn(u32, u32, *mut VciBoardInfo) -> i32,
}
impl CanLibrary {
    /// 載入 DLL 並取得所有所需的函數指標
    pub fn new(_dll_name: &str) -> Arc<Self> {
        let lib = Arc::new(unsafe { Library::new(_dll_name) }.expect("DLL load failed"));
        unsafe {
            Arc::new(Self {
                _lib: lib.clone(),
                vci_open_device: *lib.get(b"VCI_OpenDevice").expect("Failed to get VCI_OpenDevice"),
                vci_close_device: *lib.get(b"VCI_CloseDevice").expect("Failed to get VCI_CloseDevice"),
                vci_init_can: *lib.get(b"VCI_InitCAN").expect("Failed to get VCI_InitCAN"),
                vci_start_can: *lib.get(b"VCI_StartCAN").expect("Failed to get VCI_StartCAN"),
                vci_transmit: *lib.get(b"VCI_Transmit").expect("Failed to get VCI_Transmit"),
                vci_receive: *lib.get(b"VCI_Receive").expect("Failed to get VCI_Receive"),
                vci_find_usb_device2: *lib.get(b"VCI_FindUsbDevice2").expect("Failed to get VCI_FindUsbDevice2"),
                vci_read_board_info: *lib.get(b"VCI_ReadBoardInfo").expect("Failed to get VCI_ReadBoardInfo"), // ✅ 新增 VCI_ReadBoardInfo
            })
        }
    }
}

struct AppState {
    can_library: Option<Arc<CanLibrary>>,
    receiving: Arc<AtomicBool>,
}

#[tauri::command]
fn open_can_device(
    dev_type: u32,
    dev_index: u32,
    app_handle: tauri::AppHandle,
    state: State<Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let can_lib = CanLibrary::new("ControlCAN.dll");
    let reserved = 0u32;

    unsafe {
        if (can_lib.vci_open_device)(dev_type, dev_index, reserved) != 1 {
            let error_message = "開啟 CAN 裝置失敗".to_string();
            app_handle.emit("error-message", error_message.clone()).unwrap_or_default();
            return Err(error_message);
        }
    }

    println!("Device opened successfully");

    let mut app_state = state.lock().map_err(|_| "Failed to lock state")?;
    app_state.can_library = Some(can_lib);
    drop(app_state);

    Ok("CAN device opened and started successfully".into())
}

#[tauri::command]
fn stop_can_device(
    dev_type: u32,
    dev_index: u32,
    app_handle: tauri::AppHandle,
    state: State<Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let mut app_state = state.lock().map_err(|_| "Failed to lock state")?;
    if let Some(ref can_lib) = app_state.can_library {
        unsafe {
            (can_lib.vci_close_device)(dev_type, dev_index);
        }
        app_state.can_library = None;
        return Ok("CAN device stopped successfully".into());
    }
    let error_message = "CAN 裝置尚未初始化".to_string();
    app_handle.emit("error-message", error_message.clone()).unwrap_or_default();
    Err(error_message)
}

#[tauri::command]
fn start_receiving_data(
    app_handle: tauri::AppHandle,
    dev_type: u32,
    dev_index: u32,
    can_channel: u32,
    state: State<Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let state_clone = state.inner().clone();
    let receiving_flag = {
        let state_guard = state.lock().map_err(|_| "Failed to lock state")?;
        state_guard.receiving.clone()
    };
    receiving_flag.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        while receiving_flag.load(Ordering::SeqCst) {
            let message_opt = match state_clone.lock() {
                Ok(state_guard) => {
                    if let Some(ref can_lib) = state_guard.can_library {
                        let mut can_obj = VciCanObj::default();
                        let received_frames = unsafe {
                            (can_lib.vci_receive)(dev_type, dev_index, can_channel, &mut can_obj, 1, 500)
                        };
                        if received_frames > 0 {
                            let data = &can_obj.data[..(can_obj.data_len as usize)];
                            Some(format!("Received CAN message: ID=0x{:X}, Data={:?}", can_obj.id, data))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };
            if let Some(msg) = message_opt {
                let _ = app_handle.emit("can-data", msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });
    Ok(())
}


#[tauri::command]
fn stop_receiving_data(state: State<Arc<Mutex<AppState>>>) -> Result<String, String> {
    let state_guard = state.lock().map_err(|_| "Failed to lock state")?;
    state_guard.receiving.store(false, Ordering::SeqCst);
    Ok("Stopped receiving CAN data".into())
}



#[tauri::command]
fn transmit_can_data(
    data: u8,
    dev_type: u32,
    dev_index: u32,
    can_channel: u32,
    app_handle: tauri::AppHandle,
    state: State<Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let app_state = state.lock().map_err(|_| "Failed to lock state")?;
    if let Some(ref can_lib) = app_state.can_library {
        let can_obj = VciCanObj {
            id: 0x1,
            data_len: 1,
            data: [data, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        unsafe {
            let sent_frames = (can_lib.vci_transmit)(dev_type, dev_index, can_channel, &can_obj, 1);
            if sent_frames > 0 {
                return Ok(format!("Sent data: {}", data));
            } else {
                let error_message = "傳送 CAN 數據失敗".to_string();
                app_handle.emit("error-message", error_message.clone()).unwrap_or_default();
                return Err(error_message);
            }
        }
    }
    let error_message = "CAN 裝置尚未初始化".to_string();
    app_handle.emit("error-message", error_message.clone()).unwrap_or_default();
    Err(error_message)
}

#[tauri::command]
fn read_board_info(dev_type: u32, dev_index: u32, state: State<Arc<Mutex<AppState>>>) -> Result<DeviceInfo, String> {
    let app_state = state.lock().map_err(|_| "Failed to lock state")?;
    if let Some(ref can_lib) = app_state.can_library {
        let mut board_info = VciBoardInfo::default();
        unsafe {
            let status = (can_lib.vci_read_board_info)(dev_type, dev_index, &mut board_info);
            if status != 1 {
                return Err("Failed to read board info".to_string());
            }
        }
        Ok(DeviceInfo {
            index: dev_index as i32,
            serial_number: String::from_utf8_lossy(&board_info.str_serial_num).trim_matches('\0').to_string(),
            firmware_version: board_info.fw_version,
        })
    } else {
        Err("CAN library not initialized".to_string())
    }
}

#[tauri::command]
fn set_baud_rate(dev_type: u32, dev_index: u32, can_channel: u32, timing0: u8, timing1: u8, state: State<Arc<Mutex<AppState>>>) -> Result<String, String> {
    let app_state = state.lock().map_err(|_| "Failed to lock state")?;
    if let Some(ref can_lib) = app_state.can_library {
        let config = VciInitConfig {
            acc_code: 0,
            acc_mask: 0xFFFFFFFF,
            reserved: 0,
            filter: 1,
            timing0,
            timing1,
            mode: 0,
        };
        unsafe {
            if (can_lib.vci_init_can)(dev_type, dev_index, can_channel, &config) != 1 {
                return Err("Failed to set baud rate".to_string());
            }
        }
        Ok("Baud rate set successfully".to_string())
    } else {
        Err("CAN library not initialized".to_string())
    }
}

#[tauri::command]
fn reconnect_can_device(
    dev_type: u32,
    dev_index: u32,
    can1: u32,
    can2: u32,
    timing0: u8,
    timing1: u8,
    state: State<Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    {
        let mut app_state = state.lock().map_err(|_| "Failed to lock state")?;
        if let Some(ref can_lib) = app_state.can_library {
            unsafe {
                (can_lib.vci_close_device)(dev_type, dev_index);
            }
        }
        app_state.can_library = None;
    }
    let can_lib = CanLibrary::new("ControlCAN.dll");
    let reserved = 0u32;
    unsafe {
        if (can_lib.vci_open_device)(dev_type, dev_index, reserved) != 1 {
            return Err("Failed to open device".into());
        }
    }
    println!("Device reopened successfully");
    let config = VciInitConfig {
        acc_code: 0,
        acc_mask: 0xFFFFFFFF,
        reserved: 0,
        filter: 1,
        timing0,
        timing1,
        mode: 0,
    };
    unsafe {
        if (can_lib.vci_init_can)(dev_type, dev_index, can1, &config) != 1 {
            return Err("Failed to initialize CAN1 with new baud".into());
        }
        if (can_lib.vci_init_can)(dev_type, dev_index, can2, &config) != 1 {
            return Err("Failed to initialize CAN2 with new baud".into());
        }
        if (can_lib.vci_start_can)(dev_type, dev_index, can1) != 1 {
            return Err("Failed to start CAN1 after reconnect".into());
        }
        if (can_lib.vci_start_can)(dev_type, dev_index, can2) != 1 {
            return Err("Failed to start CAN2 after reconnect".into());
        }
    }
    println!("CAN channels reinitialized and started with new baud");
    {
        let mut app_state = state.lock().map_err(|_| "Failed to lock state")?;
        app_state.can_library = Some(can_lib);
    }
    Ok(format!(
        "Device reconnected with new baud: Timing0 = 0x{:X}, Timing1 = 0x{:X}",
        timing0, timing1
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState {
            can_library: None,
            receiving: Arc::new(AtomicBool::new(false)),
        })))
        .invoke_handler(tauri::generate_handler![
            open_can_device,
            stop_can_device,
            transmit_can_data,
            start_receiving_data,
            stop_receiving_data ,
            read_board_info,
            set_baud_rate,
            reconnect_can_device
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
