fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=icon.rc");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // результат компиляции ресурсов не нужен, но падать на ошибке не должен
        let _ = embed_resource::compile("icon.rc", embed_resource::NONE);
    }
}
