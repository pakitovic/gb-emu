use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x00; // no external RAM
    rom
}

fn unique_temp_file_path(name: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("gb_emu_cli_{name}_{pid}_{nanos}.{ext}"))
}

#[test]
fn cart_info_flag_prints_metadata_and_exits_successfully() {
    let rom_path = unique_temp_file_path("cart_info", "gb");
    fs::write(&rom_path, make_rom_32kb()).expect("test ROM write should succeed");

    let output = Command::new(env!("CARGO_BIN_EXE_gb-emu"))
        .arg("--cart-info")
        .arg(&rom_path)
        .output()
        .expect("CLI should execute");

    let _ = fs::remove_file(&rom_path);

    assert!(
        output.status.success(),
        "CLI should exit with success for --cart-info"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty: {stderr}");
    assert!(stdout.contains("Cartridge Metadata"));
    assert!(stdout.contains("Type: 0x00 (ROM-only)"));
    assert!(stdout.contains("Header warnings ("));
    assert!(stdout.contains("- Nintendo logo mismatch"));
}
