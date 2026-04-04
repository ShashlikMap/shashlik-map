fn main() {
    unsafe { std::env::set_var("SLINT_STYLE", "material-dark"); }
    slint_build::compile("ui/scene.slint").expect("Slint build failed");
}
