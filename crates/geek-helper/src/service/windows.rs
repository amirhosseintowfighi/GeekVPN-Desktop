//! The Windows service (LocalSystem, automatic start).

use std::ffi::{OsStr, OsString};
use std::sync::mpsc;
use std::time::Duration;

use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceInfo, ServiceStartType, ServiceState,
    ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::copy_binaries;
use crate::paths::{install_dir, HELPER_FILE};

const NAME: &str = "GeekVPNHelper";

windows_service::define_windows_service!(ffi_service_main, service_main);

pub fn run_service() -> Result<(), String> {
    service_dispatcher::start(NAME, ffi_service_main).map_err(|e| e.to_string())
}

fn service_main(_args: Vec<OsString>) {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let Ok(status) = service_control_handler::register(NAME, handler) else { return };
    let report = |state, accept| {
        let _ = status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        });
    };
    report(ServiceState::Running, ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN);
    if let Ok(rt) = super::runtime() {
        let stopped = async move {
            let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
        };
        if let Err(e) = rt.block_on(crate::server::serve(stopped)) {
            eprintln!("geekvpn-helper: {e}");
        }
    }
    report(ServiceState::Stopped, ServiceControlAccept::empty());
}

fn info() -> ServiceInfo {
    ServiceInfo {
        name: OsString::from(NAME),
        display_name: OsString::from("GeekVPN Helper"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: install_dir().join(HELPER_FILE),
        launch_arguments: vec![OsString::from("run")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    }
}

fn stop_and_wait(service: &windows_service::service::Service) {
    if service.stop().is_err() {
        return;
    }
    for _ in 0..50 {
        match service.query_status() {
            Ok(s) if s.current_state == ServiceState::Stopped => return,
            Err(_) => return,
            _ => std::thread::sleep(Duration::from_millis(200)),
        }
    }
}

pub fn install() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)
        .map_err(|e| format!("service manager: {e}"))?;
    let access = ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::STOP | ServiceAccess::CHANGE_CONFIG;
    let existing = manager.open_service(NAME, access).ok();
    if let Some(s) = &existing {
        // The running binary cannot be replaced while it runs.
        stop_and_wait(s);
    }
    copy_binaries()?;
    let service = match existing {
        Some(s) => {
            s.change_config(&info()).map_err(|e| format!("change service: {e}"))?;
            s
        }
        None => manager.create_service(&info(), access).map_err(|e| format!("create service: {e}"))?,
    };
    let _ = service.set_description("TUN mode and the kill switch for GeekVPN");
    service.start(&[] as &[&OsStr]).map_err(|e| format!("start service: {e}"))
}

pub fn uninstall() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT).map_err(|e| format!("service manager: {e}"))?;
    if let Ok(s) = manager.open_service(NAME, ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE) {
        stop_and_wait(&s);
        s.delete().map_err(|e| format!("delete service: {e}"))?;
    }
    let _ = std::fs::remove_dir_all(install_dir());
    Ok(())
}
