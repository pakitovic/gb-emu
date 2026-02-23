use std::process::Command;

#[test]
fn incompatible_blargg_and_mooneye_flags_exit_with_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_gb-emu"))
        .args(["--blargg", "--mooneye", "dummy.gb"])
        .output()
        .expect("CLI should execute");

    assert!(
        !output.status.success(),
        "CLI should fail for incompatible flags"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Use either --blargg or --mooneye"));
}

#[test]
fn invalid_model_flag_value_exits_with_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_gb-emu"))
        .args(["--model", "cgb", "dummy.gb"])
        .output()
        .expect("CLI should execute");

    assert!(
        !output.status.success(),
        "CLI should fail for invalid model value"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported model 'cgb'"));
}
