use wesl::Feature;

fn main() {
    let mut wesl = wesl::Wesl::new("src/shaders");
    wesl.set_feature("CASTANO", Feature::Enable);
    wesl.set_feature("OUTLINE_DEBUG", Feature::Disable);
    wesl.build_artifact(&"package::shape_shader".parse().unwrap(), "shape_shader");
    wesl.build_artifact(&"package::g_buf_frag_shader".parse().unwrap(), "g_buf_frag_shader");
    wesl.build_artifact(&"package::mesh_shader".parse().unwrap(), "mesh_shader");
    wesl.build_artifact(&"package::shadow_map".parse().unwrap(), "shadow_map");
    wesl.build_artifact(&"package::screen_mesh_shader".parse().unwrap(), "screen_mesh_shader");
    wesl.build_artifact(&"package::shape_culling".parse().unwrap(), "shape_culling");
    wesl.build_artifact(&"package::ssao".parse().unwrap(), "ssao");
}
