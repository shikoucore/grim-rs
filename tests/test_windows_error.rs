use grim_rs::Error;

#[test]
fn directx_error_displays_correctly() {
    let err = Error::DirectXError("CreateDXGIFactory1: HRESULT 0x80070057".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("DirectX error"));
    assert!(msg.contains("CreateDXGIFactory1"));
    assert!(msg.contains("0x80070057"));
}

#[test]
fn directx_error_empty_message() {
    let err = Error::DirectXError(String::new());
    assert_eq!(format!("{}", err), "DirectX error: ");
}

#[test]
fn protected_content_displays_correctly() {
    let err = Error::ProtectedContent("Output 'DISPLAY1' is protected by DRM".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Protected content"));
    assert!(msg.contains("DISPLAY1"));
    assert!(msg.contains("DRM"));
}

#[test]
fn no_gpu_adapter_displays_correctly() {
    let err = Error::NoGpuAdapter;
    let msg = format!("{}", err);
    assert!(msg.contains("GPU"));
    assert!(msg.contains("adapter"));
    assert!(!msg.is_empty());
}

#[test]
fn error_variants_are_debug_printable() {
    let errors = [
        Error::DirectXError("test".into()),
        Error::ProtectedContent("test".into()),
        Error::NoGpuAdapter,
    ];
    for err in &errors {
        let debug = format!("{:?}", err);
        assert!(!debug.is_empty());
    }
}
