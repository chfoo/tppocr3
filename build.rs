use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=vncserver");

    for name in &["libvnc"] {
        println!("cargo:rerun-if-changed=src/c/{}_wrapper.hpp", name);

        let bindings = bindgen::Builder::default()
            .header(format!("src/c/{}_wrapper.hpp", name))
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings");

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join(format!("{}_bindings.rs", name)))
            .expect("Couldn't write bindings!");
    }
}
