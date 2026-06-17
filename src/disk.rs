use std::process::Command;

#[derive(Debug, Clone)]
pub struct Disk {
    pub name: String,
    pub size: String,
    pub model: String,
    pub path: String,
}

pub fn list_disks() -> Vec<Disk> {
    let mut disks = Vec::new();
    let output = Command::new("lsblk")
        .args(["-d", "-o", "NAME,SIZE,MODEL", "-p", "-n", "-l"])
        .output()
        .ok();
    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
            if parts.len() >= 2 {
                disks.push(Disk {
                    path: parts[0].to_string(),
                    name: parts[0].split('/').last().unwrap_or("").to_string(),
                    size: parts[1].to_string(),
                    model: parts.get(2).unwrap_or(&"").trim().to_string(),
                });
            }
        }
    }
    disks
}

pub fn run_cfdisk(disk: &str) -> bool {
    let status = Command::new("cfdisk")
        .arg(disk)
        .status()
        .expect("failed to run cfdisk");
    status.success()
}

pub fn guided_partition(disk: &str) -> bool {
    let wipe = Command::new("sgdisk")
        .args(["-Z", disk])
        .status()
        .ok();
    if wipe.map_or(true, |s| !s.success()) {
        let _ = Command::new("dd")
            .args(["if=/dev/zero", "of=", disk, "bs=1M", "count=1"])
            .status();
    }

    let gpt = Command::new("sgdisk")
        .args(["-o", disk])
        .status()
        .ok();
    if gpt.map_or(true, |s| !s.success()) {
        return false;
    }

    let efi = Command::new("sgdisk")
        .args(["-n", "1:0:+512M", "-t", "1:ef00", disk])
        .status()
        .ok();
    if efi.map_or(true, |s| !s.success()) {
        return false;
    }

    let root = Command::new("sgdisk")
        .args(["-n", "2:0:0", "-t", "2:8300", disk])
        .status()
        .ok();
    if root.map_or(true, |s| !s.success()) {
        return false;
    }

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = Command::new("partprobe").arg(disk).status();
    std::thread::sleep(std::time::Duration::from_millis(500));

    true
}

pub fn get_partitions(disk: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let output = Command::new("lsblk")
        .args(["-o", "NAME", "-p", "-n", "-l", disk])
        .output()
        .ok();
    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let p = line.trim().to_string();
            if !p.is_empty() && p != disk {
                parts.push(p);
            }
        }
    }
    parts
}

pub fn format_partition(part: &str, fstype: &str) -> bool {
    match fstype {
        "ext4" => {
            let status = Command::new("mkfs.ext4")
                .args(["-F", part])
                .status()
                .ok();
            status.map_or(false, |s| s.success())
        }
        "btrfs" => {
            let status = Command::new("mkfs.btrfs")
                .args(["-f", part])
                .status()
                .ok();
            status.map_or(false, |s| s.success())
        }
        "vfat" => {
            let status = Command::new("mkfs.fat")
                .args(["-F32", part])
                .status()
                .ok();
            status.map_or(false, |s| s.success())
        }
        _ => false,
    }
}

pub fn mount_partition(part: &str, mountpoint: &str) -> bool {
    let _ = std::fs::create_dir_all(mountpoint);
    let status = Command::new("mount")
        .args([part, mountpoint])
        .status()
        .ok();
    status.map_or(false, |s| s.success())
}

pub fn unmount_all(root: &str) {
    let _ = Command::new("umount").args(["-R", root]).status();
}

pub fn is_root() -> bool {
    let output = Command::new("id").arg("-u").output().ok();
    if let Some(output) = output {
        let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        uid == "0"
    } else {
        false
    }
}
