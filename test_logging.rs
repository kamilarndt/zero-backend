use std::path::PathBuf;
use directories::{ProjectDirs, BaseDirs};

fn main() {
    // Test ProjectDirs
    let proj_dirs = ProjectDirs::from("com", "zeroclaw", "zeroclaw");
    println!("ProjectDirs: {:?}", proj_dirs);
    
    if let Some(proj_dirs) = proj_dirs {
        println!("Config dir: {:?}", proj_dirs.config_dir());
        println!("Log dir would be: {:?}", proj_dirs.config_dir().join("logs"));
    }
    
    // Test BaseDirs
    let base_dirs = BaseDirs::new();
    println!("BaseDirs: {:?}", base_dirs);
    
    if let Some(base_dirs) = base_dirs {
        println!("Home dir: {:?}", base_dirs.home_dir());
        println!("Fallback log dir: {:?}", base_dirs.home_dir().join(".zeroclaw/logs"));
    }
    
    // Test PathBuf fallback
    let fallback = PathBuf::from(".");
    println!("Fallback to current dir: {:?}", fallback);
}
