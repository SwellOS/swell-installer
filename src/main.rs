#![allow(dead_code)]

mod boot;
mod config;
mod disk;
mod install;

use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::process::Command;

const ROOT_MOUNT: &str = "/mnt/swell";

fn main() {
    println!("\n  SwellOS Installer v0.1");
    println!("  -----------------------\n");

    if !disk::is_root() {
        eprintln!("error: installer must be run as root");
        std::process::exit(1);
    }

    // Step 1: Select disk
    let disks = disk::list_disks();
    if disks.is_empty() {
        eprintln!("error: no disks found");
        std::process::exit(1);
    }

    let disk_names: Vec<String> = disks.iter().map(|d| {
        if d.model.is_empty() {
            format!("{} ({})", d.path, d.size)
        } else {
            format!("{} ({} - {})", d.path, d.size, d.model)
        }
    }).collect();

    let disk_idx = Select::new()
        .with_prompt("Select installation disk")
        .items(&disk_names)
        .default(0)
        .interact()
        .expect("failed to read selection");

    let selected_disk = &disks[disk_idx].path;

    // Step 2: Partitioning method
    let part_options = vec!["Guided (use entire disk)", "Manual (launch cfdisk)"];
    let part_method = Select::new()
        .with_prompt("Partitioning method")
        .items(&part_options)
        .default(0)
        .interact()
        .expect("failed to read selection");

    let success = if part_method == 0 {
        println!("\nPartitioning {}...", selected_disk);
        if !Confirm::new()
            .with_prompt("This will erase ALL data on this disk. Continue?")
            .default(false)
            .interact()
            .unwrap()
        {
            println!("Aborted.");
            std::process::exit(0);
        }
        disk::guided_partition(selected_disk)
    } else {
        println!("\nLaunching cfdisk for {}...", selected_disk);
        disk::run_cfdisk(selected_disk)
    };

    if !success {
        eprintln!("error: partitioning failed");
        std::process::exit(1);
    }

    // Step 3: Detect partitions
    let parts = disk::get_partitions(selected_disk);
    if parts.len() < 1 {
        eprintln!("error: no partitions found after partitioning");
        std::process::exit(1);
    }

    let root_part = if parts.len() >= 2 {
        // Assume last partition is root
        parts.last().unwrap().clone()
    } else {
        parts[0].clone()
    };

    let efi_part = if boot::detect_efi() && parts.len() >= 2 {
        Some(parts[0].clone())
    } else {
        None
    };

    // Step 4: Select filesystem
    let fs_options = vec!["ext4", "btrfs"];
    let fs_idx = Select::new()
        .with_prompt("Select filesystem")
        .items(&fs_options)
        .default(0)
        .interact()
        .expect("failed to read selection");

    let fstype = fs_options[fs_idx];

    // Step 5: Format partitions
    println!("\nFormatting partitions...");

    if let Some(efi) = &efi_part {
        println!("  Formatting EFI partition: {}", efi);
        if !disk::format_partition(efi, "vfat") {
            eprintln!("error: failed to format EFI partition");
            std::process::exit(1);
        }
    }

    println!("  Formatting root partition: {} as {}", root_part, fstype);
    if !disk::format_partition(&root_part, fstype) {
        eprintln!("error: failed to format root partition");
        std::process::exit(1);
    }

    // Step 6: Mount
    println!("\nMounting partitions...");
    let _ = std::fs::create_dir_all(ROOT_MOUNT);

    if !disk::mount_partition(&root_part, ROOT_MOUNT) {
        eprintln!("error: failed to mount root partition");
        std::process::exit(1);
    }

    if let Some(efi) = &efi_part {
        let efi_dir = format!("{}/boot/efi", ROOT_MOUNT);
        let _ = std::fs::create_dir_all(&efi_dir);
        if !disk::mount_partition(efi, &efi_dir) {
            eprintln!("error: failed to mount EFI partition");
            std::process::exit(1);
        }
    }

    // Step 7: Select metapackages
    let metapackage_options = vec![
        "kde-plasma           KDE Plasma 6 desktop on Wayland",
        "gnome                GNOME 47 desktop on Wayland",
        "xfce4                XFCE 4.20 lightweight desktop",
        "hyprland             Hyprland Wayland compositor (dynamic tiling)",
        "i3                   i3 tiling window manager",
        "windowmaker          WindowMaker window manager (NeXTSTEP-like)",
        "swell-dev            Development tools (GCC, LLVM, git, Python, Rust)",
        "swell-browser        Firefox browser (hardened build)",
        "swell-gaming         Gaming (Steam runtime, wine, DXVK)",
        "swell-multimedia     Multimedia (FFmpeg, GIMP, OBS, Blender)",
        "swell-office         Office (LibreOffice, Thunderbird)",
    ];
    let mpkg_labels: Vec<&str> = metapackage_options.iter().map(|s| *s).collect();

    let selections = MultiSelect::new()
        .with_prompt("Select metapackages to install (use Space to toggle, Enter to confirm)")
        .items(&mpkg_labels)
        .interact()
        .expect("failed to read selections");

    let selected_metapackages: Vec<String> = selections.iter().map(|i| {
        let label = metapackage_options[*i];
        label.split_whitespace().next().unwrap_or("").to_string()
    }).collect();

    println!("\nSelected: {}", if selected_metapackages.is_empty() {
        "none (CLI only)".to_string()
    } else {
        selected_metapackages.join(", ")
    });

    // Step 8: Build system
    println!("\n=== Building System ===");
    println!("This will download and compile all packages from source.");
    println!("This may take a long time (30 minutes to several hours).");

    if !Confirm::new()
        .with_prompt("Start build?")
        .default(true)
        .interact()
        .unwrap()
    {
        println!("Aborted.");
        disk::unmount_all(ROOT_MOUNT);
        std::process::exit(0);
    }

    let installer = install::Installer::new(ROOT_MOUNT);

    if let Err(e) = installer.copy_to_root() {
        eprintln!("error: failed to set up root: {}", e);
        disk::unmount_all(ROOT_MOUNT);
        std::process::exit(1);
    }

    if let Err(e) = installer.build_core_system() {
        eprintln!("error: core system build failed: {}", e);
        disk::unmount_all(ROOT_MOUNT);
        std::process::exit(1);
    }

    if !selected_metapackages.is_empty() {
        let pkg_installer = install::Installer {
            root_mount: ROOT_MOUNT.to_string(),
            selected_packages: selected_metapackages.clone(),
        };
        if let Err(e) = pkg_installer.build_selected_packages() {
            eprintln!("warning: some metapackages failed: {}", e);
        }
    }

    // Step 9: Configure system
    println!("\n=== System Configuration ===");

    let hostname: String = Input::new()
        .with_prompt("Hostname")
        .default("swell".to_string())
        .interact_text()
        .expect("failed to read hostname");

    let root_password: String = Input::new()
        .with_prompt("Root password")
        .default("swell".to_string())
        .interact_text()
        .expect("failed to read password");

    let create_user = Confirm::new()
        .with_prompt("Create a regular user?")
        .default(true)
        .interact()
        .unwrap();

    let username = if create_user {
        let uname: String = Input::new()
            .with_prompt("Username")
            .default("user".to_string())
            .interact_text()
            .expect("failed to read username");

        let upass: String = Input::new()
            .with_prompt("User password")
            .default("user".to_string())
            .interact_text()
            .expect("failed to read password");

        let _ = config::create_user(ROOT_MOUNT, &uname, &upass, &["wheel"]);
        uname
    } else {
        String::new()
    };

    let timezone: String = Input::new()
        .with_prompt("Timezone (e.g. Europe/Warsaw, America/New_York)")
        .default("UTC".to_string())
        .interact_text()
        .expect("failed to read timezone");

    let _ = config::set_hostname(ROOT_MOUNT, &hostname);
    let _ = config::set_root_password(ROOT_MOUNT, &root_password);
    let _ = config::set_timezone(ROOT_MOUNT, &timezone);
    let _ = config::configure_network(ROOT_MOUNT);
    let _ = config::configure_fstab(ROOT_MOUNT, &root_part, efi_part.as_deref(), fstype);

    // Step 10: Install GRUB
    println!("\n=== Bootloader ===");
    if boot::detect_efi() {
        if let Err(e) = boot::install_grub(ROOT_MOUNT, selected_disk) {
            eprintln!("error: GRUB installation failed: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("BIOS boot not yet supported. Install GRUB manually.");
    }

    // Step 11: Done
    println!("\n=== Installation Complete ===");
    println!("\nSwellOS has been installed to {}", ROOT_MOUNT);
    if !username.is_empty() {
        println!("  Hostname: {}", hostname);
        println!("  User: {}", username);
    }

    if Confirm::new()
        .with_prompt("Reboot now?")
        .default(true)
        .interact()
        .unwrap()
    {
        println!("Rebooting...");
        disk::unmount_all(ROOT_MOUNT);
        let _ = Command::new("reboot").status();
    } else {
        println!("You can reboot later with: reboot");
        println!("Target system is mounted at: {}", ROOT_MOUNT);
    }
}
