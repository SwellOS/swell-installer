use std::path::Path;
use std::process::Command;

pub fn install_grub(root: &str, _disk: &str) -> Result<(), String> {
    println!("Installing GRUB...");

    // Mount necessary filesystems in chroot
    let dirs = ["dev", "proc", "sys", "run"];
    for d in &dirs {
        let target = format!("{}/{}", root, d);
        let _ = std::fs::create_dir_all(&target);
        let _ = Command::new("mount")
            .args(["--bind", &format!("/{}", d), &target])
            .status();
    }

    // Install GRUB for EFI
    let status = Command::new("chroot")
        .args([root, "grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--bootloader-id=SwellOS", "--recheck"])
        .status()
        .map_err(|e| format!("failed to run grub-install: {}", e))?;

    if !status.success() {
        return Err("grub-install failed".to_string());
    }

    // Generate GRUB config
    let config_status = Command::new("chroot")
        .args([root, "grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
        .status()
        .map_err(|e| format!("failed to run grub-mkconfig: {}", e))?;

    if !config_status.success() {
        return Err("grub-mkconfig failed".to_string());
    }

    println!("GRUB installed successfully.");
    Ok(())
}

pub fn detect_efi() -> bool {
    Path::new("/sys/firmware/efi").exists()
}
