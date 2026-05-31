fn main() {
    built::write_built_file()
        .expect("built failed to acquire build metadata");
}
