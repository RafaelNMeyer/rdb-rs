use std::process::Command;

fn main() {
    Command::new("gcc")
        .args(&["test/targets/reg_write.s", "-pie", "-o"])
        .arg(&format!("{}/reg_write", "target"))
        .status()
        .unwrap();

    cc::Build::new()
        .cpp(true)           // Compile as C++
        .file("src/bindings_decl.cpp")
        .compile("bindings_decl"); // Output library name

    println!("cargo::rerun-if-changed=test/targets/reg_write.s");
}
