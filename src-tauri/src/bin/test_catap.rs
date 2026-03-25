use laconote_desktop_lib::permissions::{check_permissions, PermissionState};

fn main() {
    println!("Testing CATap permission...");
    let granted = check_permissions().system_audio == PermissionState::Granted;
    println!("System audio permission returned: {}", granted);
}
