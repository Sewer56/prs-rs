fn main() {
    csbindgen::Builder::default()
        .input_extern_file("src/exports.rs")
        .csharp_dll_name("prs_rs")
        .csharp_class_accessibility("public")
        .csharp_namespace("prs_rs.Net.Sys")
        .generate_csharp_file("bindings/csharp/NativeMethods.g.cs")
        .unwrap();
}
