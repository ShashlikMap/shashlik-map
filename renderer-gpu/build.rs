use wesl::Feature;

fn main() {
    let mut wesl = wesl::Wesl::new("src/shaders");
    wesl.set_feature("CASTANO", Feature::Enable);
    wesl.build_artifact(&"package::shape_shader".parse().unwrap(), "shape_shader");
    wesl.build_artifact(&"package::mesh_shader".parse().unwrap(), "mesh_shader");
    wesl.build_artifact(&"package::screen_mesh_shader".parse().unwrap(), "screen_mesh_shader");
    wesl.build_artifact(&"package::shape_culling".parse().unwrap(), "shape_culling");
    wesl.build_artifact(&"package::ssao".parse().unwrap(), "ssao");
}
