#![cfg(not(target_os = "linux"))]

use pvr_compiler::{AVAILABLE, Compiler};

#[test]
fn unsupported_target_reports_unavailability() {
    let available = AVAILABLE;
    assert!(!available);
    assert_eq!(
        Compiler::new().err().as_deref(),
        Some("PVR compiler is only available on Linux targets")
    );
}
