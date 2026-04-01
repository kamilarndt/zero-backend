use std::env;

// Set environment variable
env::set_var("SIYUAN_API_TOKEN", "");

// Now try to create the tool
fn main() {
    println!("Testing SiyuanQueryTool creation...");
    match zeroclaw_tools::SiyuanQueryTool::new() {
        Ok(tool) => println!("SUCCESS: Tool created: {}", tool.name()),
        Err(e) => println!("FAILED: {}", e),
    }
}
