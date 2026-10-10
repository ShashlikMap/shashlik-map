use wesl::{CompileOptions, Compiler, Feature};

const SHADERS: [&str; 8] = [
    "shape_shader",
    "g_buf_frag_shader",
    "mesh_shader",
    "shadow_map",
    "screen_mesh_shader",
    "shape_culling",
    "ssao",
    "x_real_mesh_shader",
];

fn main() {
    let mut options = CompileOptions::default();
    options.features.set("CASTANO", Feature::Enable);
    options.features.set("OUTLINE_DEBUG", Feature::Disable);
    let compiler = Compiler::new(options);
    for shader in SHADERS {
        let res = compiler
            .compile(format!("src/shaders/{shader}.wgsl"))
            .unwrap_or_else(|e| panic!("failed to compile {shader}: {e}"));
        res.emit_rerun_if_changed();
        res.write_artifact(shader);
    }
}
