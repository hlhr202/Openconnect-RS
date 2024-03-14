fn main() {
    // this requires signing the app
    let _windows_attributes = tauri_build::WindowsAttributes::new().app_manifest(
        r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="true" />
        </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#,
    );

    // this requires signing the app
    // tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows_attributes))
    //     .expect("error while building tauri application")

    tauri_build::build();
}
