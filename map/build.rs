fn main() {
    dotenvy::dotenv().ok();
    if let Ok(key) = std::env::var("MAPTILER_API_KEY") {
        println!("cargo:rustc-env=MAPTILER_API_KEY={}", key);
    } else {
        panic!("Error: MAPTILER_API_KEY environment variable or .env file is missing!");
    }
}
