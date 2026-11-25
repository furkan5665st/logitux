fn main() {
    println!("cargo:rerun-if-changed=../../assets/logitux.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/logitux.ico");
        res.set("ProductName", "LogiTux");
        res.set("FileDescription", "LogiTux");
        let _ = res.compile();
    }
}
