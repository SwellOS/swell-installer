use std::path::Path;
use std::process::Command;

const REPO_PATH: &str = "/usr/src/swell/packages";

pub struct Installer {
    pub root_mount: String,
    pub selected_packages: Vec<String>,
}

impl Installer {
    pub fn new(root_mount: &str) -> Self {
        Self {
            root_mount: root_mount.to_string(),
            selected_packages: Vec::new(),
        }
    }

    pub fn build_core_system(&self) -> Result<(), String> {
        println!("Building core system packages...");

        let core_packages = vec![
            "linux-api-headers",
            "glibc",
            "gcc",
            "binutils",
            "dash",
            "coreutils",
            "make",
            "sed",
            "grep",
            "gawk",
            "file",
            "diffutils",
            "findutils",
            "patch",
        ];

        for pkg in &core_packages {
            println!("  Building: {}", pkg);
            let status = Command::new("sbpm")
                .args(["-S", pkg, "--source-only"])
                .env("SWELL_ROOT", &self.root_mount)
                .status()
                .map_err(|e| format!("failed to run sbpm: {}", e))?;

            if !status.success() {
                return Err(format!("failed to build core package: {}", pkg));
            }
        }

        // Install runit and sbpm itself
        println!("  Building: runit");
        let _ = Command::new("sbpm").args(["-S", "runit"]).status();

        println!("Core system build complete.");
        Ok(())
    }

    pub fn build_selected_packages(&self) -> Result<(), String> {
        if self.selected_packages.is_empty() {
            println!("No additional packages selected.");
            return Ok(());
        }

        println!("Building selected packages...");
        for pkg in &self.selected_packages {
            if self.is_metapackage(pkg) {
                println!("  Resolving metapackage: {}", pkg);
                let status = Command::new("sbpm")
                    .args(["-S", pkg])
                    .env("SWELL_ROOT", &self.root_mount)
                    .status()
                    .map_err(|e| format!("failed to install {}: {}", pkg, e))?;
                if !status.success() {
                    println!("  Warning: {} install had issues", pkg);
                }
            } else {
                println!("  Building: {}", pkg);
                let status = Command::new("sbpm")
                    .args(["-S", pkg])
                    .env("SWELL_ROOT", &self.root_mount)
                    .status()
                    .map_err(|e| format!("failed to build {}: {}", pkg, e))?;
                if !status.success() {
                    println!("  Warning: {} build had issues", pkg);
                }
            }
        }
        println!("Package build complete.");
        Ok(())
    }

    fn is_metapackage(&self, name: &str) -> bool {
        let meta_path = Path::new(REPO_PATH).join("meta").join(name);
        meta_path.exists() && meta_path.is_dir()
    }

    pub fn copy_to_root(&self) -> Result<(), String> {
        println!("Copying built system to root...");
        // This is handled by sbpm's DESTDIR mechanism
        // We just need to ensure /dev, /proc, /sys are mounted in the target
        let dirs = ["dev", "proc", "sys", "run"];
        for d in &dirs {
            let target = format!("{}/{}", self.root_mount, d);
            let _ = std::fs::create_dir_all(&target);
            let _ = Command::new("mount")
                .args(["--bind", &format!("/{}", d), &target])
                .status();
        }
        Ok(())
    }
}
