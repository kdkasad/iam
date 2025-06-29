use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only run the build script if we're building the server
    if env::var("CARGO_PKG_NAME").unwrap() != "iam-server" {
        return;
    }

    // Check if we're in development mode
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    if profile == "debug" {
        println!("cargo::warning=Development mode: Frontend will be served from ui/build/");
        println!("cargo::rerun-if-changed=ui/build/");
        return;
    }

    // Build the frontend
    println!("cargo::warning=Building frontend...");
    
    let ui_dir = Path::new("ui");
    if !ui_dir.exists() {
        println!("cargo::warning=UI directory not found, skipping frontend build");
        return;
    }

    // Run npm install if package.json exists
    let package_json = ui_dir.join("package.json");
    if package_json.exists() {
        let status = Command::new("npm")
            .arg("install")
            .current_dir(ui_dir)
            .status();
        
        if let Err(e) = status {
            println!("cargo::warning=Failed to run npm install: {}", e);
            return;
        }
    }

    // Run npm run build
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(ui_dir)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Frontend built successfully");
                println!("cargo::rerun-if-changed=ui/build/");
            } else {
                println!("cargo::warning=Frontend build failed with exit code: {}", exit_status);
            }
        }
        Err(e) => {
            println!("cargo::error=Failed to build frontend: {}", e);
        }
    }
} 