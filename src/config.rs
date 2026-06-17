use std::fs;
use std::path::Path;
use std::process::Command;

pub fn set_hostname(root: &str, hostname: &str) -> Result<(), String> {
    let hostname_path = Path::new(root).join("etc").join("hostname");
    fs::write(&hostname_path, format!("{}\n", hostname))
        .map_err(|e| format!("failed to write hostname: {}", e))?;

    // Also set in /etc/hosts
    let hosts_path = Path::new(root).join("etc").join("hosts");
    let hosts_content = format!(
        "127.0.0.1   localhost\n::1         localhost\n127.0.1.1   {}.localdomain {}\n",
        hostname, hostname
    );
    fs::write(&hosts_path, hosts_content)
        .map_err(|e| format!("failed to write hosts: {}", e))?;

    println!("  Hostname set to: {}", hostname);
    Ok(())
}

pub fn set_root_password(root: &str, password: &str) -> Result<(), String> {
    // Use chroot + passwd
    let status = Command::new("chroot")
        .args([root, "sh", "-c", &format!("echo 'root:{}' | chpasswd", password)])
        .status()
        .map_err(|e| format!("failed to set root password: {}", e))?;

    if !status.success() {
        return Err("failed to set root password".to_string());
    }
    println!("  Root password set.");
    Ok(())
}

pub fn create_user(root: &str, username: &str, password: &str, groups: &[&str]) -> Result<(), String> {
    // Create user
    let status = Command::new("chroot")
        .args([root, "useradd", "-m", "-G", &groups.join(","), username])
        .status()
        .map_err(|e| format!("failed to create user: {}", e))?;

    if !status.success() {
        return Err(format!("failed to create user: {}", username));
    }

    // Set password
    let pw_status = Command::new("chroot")
        .args([root, "sh", "-c", &format!("echo '{}:{}' | chpasswd", username, password)])
        .status()
        .map_err(|e| format!("failed to set user password: {}", e))?;

    if !pw_status.success() {
        return Err(format!("failed to set password for {}", username));
    }

    println!("  User created: {}", username);
    Ok(())
}

pub fn set_timezone(root: &str, timezone: &str) -> Result<(), String> {
    let tz_path = format!("/usr/share/zoneinfo/{}", timezone);
    let target = Path::new(root).join("etc").join("localtime");

    // Remove existing if any
    let _ = fs::remove_file(&target);

    // Copy timezone file
    let status = Command::new("cp")
        .args([&tz_path, &target.to_string_lossy().to_string()])
        .status()
        .map_err(|e| format!("failed to set timezone: {}", e))?;

    if !status.success() {
        return Err(format!("timezone {} not found", timezone));
    }

    // Write /etc/timezone
    let tz_file = format!("{}\n", timezone);
    let _ = fs::write(Path::new(root).join("etc").join("timezone"), &tz_file);

    println!("  Timezone set to: {}", timezone);
    Ok(())
}

pub fn configure_network(root: &str) -> Result<(), String> {
    // Enable dhcpcd service
    let sv_dir = Path::new(root).join("etc").join("sv");
    let dhcpcd_sv = sv_dir.join("dhcpcd");
    if !dhcpcd_sv.exists() {
        let _ = fs::create_dir_all(&dhcpcd_sv);
        fs::write(dhcpcd_sv.join("run"), "#!/bin/sh\nexec dhcpcd -n\n")
            .map_err(|e| format!("failed to create dhcpcd run script: {}", e))?;
        let _ = fs::write(dhcpcd_sv.join("finish"), "#!/bin/sh\nexit 0\n");
    }

    // Enable the service
    let var_sv = Path::new(root).join("var").join("service");
    let _ = fs::create_dir_all(&var_sv);
    let _ = fs::remove_file(var_sv.join("dhcpcd"));
    let _ = std::os::unix::fs::symlink(&dhcpcd_sv, var_sv.join("dhcpcd"));

    // Make scripts executable
    let _ = Command::new("chroot")
        .args([root, "sh", "-c", "chmod +x /etc/sv/dhcpcd/run /etc/sv/dhcpcd/finish 2>/dev/null"])
        .status();

    println!("  Network configured (dhcpcd enabled).");
    Ok(())
}

pub fn configure_fstab(root: &str, root_part: &str, efi_part: Option<&str>, fstype: &str) -> Result<(), String> {
    let mut fstab = String::new();
    fstab.push_str("# /etc/fstab: static file system information\n");
    fstab.push_str(&format!("{}  /  {}  defaults  0  1\n", root_part, fstype));

    if let Some(efi) = efi_part {
        fstab.push_str(&format!("{}  /boot/efi  vfat  defaults  0  2\n", efi));
    }

    fstab.push_str("proc  /proc  proc  nosuid,noexec,nodev  0  0\n");
    fstab.push_str("sys   /sys   sysfs nosuid,noexec,nodev  0  0\n");
    fstab.push_str("dev   /dev   devtmpfs  nosuid  0  0\n");
    fstab.push_str("tmpfs /run  tmpfs  nosuid,nodev  0  0\n");

    fs::write(Path::new(root).join("etc").join("fstab"), fstab)
        .map_err(|e| format!("failed to write fstab: {}", e))?;

    println!("  /etc/fstab written.");
    Ok(())
}
