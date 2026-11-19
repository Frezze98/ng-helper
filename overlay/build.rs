// Вшиваем иконку в ng-helper.exe, чтобы она отображалась
// на ярлыках, в проводнике, в «Установке и удалении программ».
fn main() {
    println!("cargo:rerun-if-changed=../installer/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../installer/icon.ico");
        res.set("ProductName", "NG Helper");
        res.set("FileDescription", "NG Helper");
        if let Err(e) = res.compile() {
            // не валим сборку из-за иконки
            println!("cargo:warning=failed to embed icon: {e}");
        }
    }
}
