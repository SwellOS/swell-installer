use clap::Parser;

#[derive(Parser)]
#[command(name = "swell-install", about = "SwellOS TUI installer")]
struct Cli;

fn main() {
    let _cli = Cli::parse();

    println!("=== SwellOS Installer ===");
    println!("1. Partition disk");
    println!("2. Select filesystem");
    println!("3. Select metapackages");
    println!("4. Build system from source");
    println!("5. Configure system");
    println!("6. Install bootloader");
    println!("7. Reboot");
    println!();

    let steps: Vec<&str> = vec![
        "Partition disk",
        "Select filesystem",
        "Select metapackages",
        "Build from source (this will take a while)",
        "Configure system",
        "Install bootloader",
    ];

    for (i, step) in steps.iter().enumerate() {
        println!("[{}/{}] {} — not yet implemented", i + 1, steps.len(), step);
    }
}
