@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, read_write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@group(0) @binding(3) var depth: texture_depth_2d;

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_proj_inv: mat4x4<f32>,
    view_tr_inv: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

var<immediate> constans: u32;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;

    if(pixel_coord.x >= u32(1.0 / camera.inv_screen_size.x) || pixel_coord.y >= u32(1.0 / camera.inv_screen_size.y)) {
        return;
    }

    let is_blur = constans != 0;
    if(is_blur) {
        compute_blur(pixel_coord);
    } else {
        compute_ssao(pixel_coord);
    }
}

const kernel = array<vec3f, 8>(
      vec3f(-0.988005f, -0.614986f, 0.277931f),
      vec3f(0.308289f, 0.470236f, 0.121804f),
      vec3f(-0.286518f, 0.318795f, 0.864876f),
      vec3f(-0.330144f, -0.274284f, 0.511152f),
      vec3f(0.477399f, -0.142303f, -0.593631f),
      vec3f(0.810853f, -0.212716f, 0.217826f),
      vec3f(-0.100139f, 0.818944f, 0.441590f),
      vec3f(0.752630f, -0.281644f, 0.447299f)
  );

const noise: array<vec3f, 256> = array<vec3f, 256>(
          vec3f(-0.667189f, 0.942519f, -0.273498f),
          vec3f(-0.987577f, 0.645436f, -0.947119f),
          vec3f(-0.474665f, 0.885778f, -0.701171f),
          vec3f(-0.486533f, 0.969479f, -0.453678f),
          vec3f(-0.526492f, 0.005675f, 0.584094f),
          vec3f(0.957280f, -0.535425f, 0.912158f),
          vec3f(0.104563f, -0.724771f, 0.547125f),
          vec3f(0.973977f, 0.813847f, 0.157363f),
          vec3f(0.896977f, 0.099820f, -0.242966f),
          vec3f(0.984254f, -0.814903f, 0.149127f),
          vec3f(0.785486f, -0.750525f, 0.957960f),
          vec3f(-0.379445f, -0.443917f, -0.639825f),
          vec3f(0.317797f, -0.321509f, -0.830390f),
          vec3f(-0.971187f, 0.845374f, -0.170327f),
          vec3f(0.585970f, -0.119611f, -0.311010f),
          vec3f(0.086937f, -0.531304f, -0.052757f),
          vec3f(-0.291528f, 0.676551f, -0.990482f),
          vec3f(0.501811f, -0.650122f, 0.589545f),
          vec3f(-0.456164f, -0.155153f, 0.279647f),
          vec3f(-0.342694f, 0.524905f, -0.087984f),
          vec3f(-0.123640f, -0.260073f, 0.469062f),
          vec3f(0.106036f, 0.520784f, -0.773590f),
          vec3f(-0.236044f, -0.143180f, -0.296658f),
          vec3f(-0.186357f, 0.349789f, -0.158644f),
          vec3f(0.651715f, 0.692696f, 0.999933f),
          vec3f(0.372982f, 0.245360f, -0.656232f),
          vec3f(0.657932f, -0.458163f, -0.890090f),
          vec3f(0.424037f, 0.605047f, -0.368335f),
          vec3f(0.060654f, -0.694266f, -0.635239f),
          vec3f(0.060912f, 0.484683f, 0.046305f),
          vec3f(-0.417268f, 0.537721f, 0.216225f),
          vec3f(-0.408837f, 0.693578f, -0.250333f),
          vec3f(-0.724172f, -0.753956f, 0.196931f),
          vec3f(0.231838f, 0.224299f, -0.384530f),
          vec3f(-0.332188f, 0.104348f, -0.554275f),
          vec3f(-0.227376f, 0.230458f, 0.207050f),
          vec3f(-0.553993f, 0.171614f, 0.796151f),
          vec3f(0.380805f, -0.834358f, 0.284473f),
          vec3f(-0.273264f, -0.121976f, 0.695990f),
          vec3f(0.972434f, 0.214833f, 0.288698f),
          vec3f(0.901028f, -0.204713f, 0.203984f),
          vec3f(-0.868960f, 0.932016f, -0.283415f),
          vec3f(-0.601750f, -0.900961f, -0.962678f),
          vec3f(0.650001f, -0.721799f, -0.365500f),
          vec3f(0.376599f, -0.031811f, 0.152510f),
          vec3f(-0.718171f, 0.616849f, 0.196559f),
          vec3f(0.391044f, 0.504265f, 0.193003f),
          vec3f(0.657868f, -0.977338f, -0.863750f),
          vec3f(-0.560902f, -0.381578f, -0.089020f),
          vec3f(-0.704922f, -0.074127f, -0.407078f),
          vec3f(0.472734f, -0.215928f, -0.229243f),
          vec3f(0.754023f, 0.260845f, 0.228767f),
          vec3f(-0.682819f, -0.993022f, 0.238840f),
          vec3f(-0.632339f, -0.781440f, -0.846763f),
          vec3f(-0.077793f, 0.071905f, 0.821009f),
          vec3f(0.884533f, -0.890968f, -0.672849f),
          vec3f(0.937114f, 0.107268f, -0.823282f),
          vec3f(0.140020f, -0.833880f, -0.436627f),
          vec3f(-0.950325f, 0.227045f, -0.724462f),
          vec3f(-0.062856f, 0.266868f, 0.532431f),
          vec3f(0.653142f, 0.947945f, 0.076574f),
          vec3f(-0.239250f, 0.210866f, -0.562606f),
          vec3f(0.293359f, -0.108774f, -0.476411f),
          vec3f(-0.440677f, -0.439867f, -0.717793f),
          vec3f(0.472820f, 0.045261f, 0.613551f),
          vec3f(0.729853f, -0.299188f, 0.602487f),
          vec3f(-0.242671f, -0.502998f, -0.866300f),
          vec3f(0.042153f, -0.234777f, 0.691247f),
          vec3f(-0.521590f, 0.249239f, 0.279308f),
          vec3f(0.791067f, 0.618762f, -0.107118f),
          vec3f(0.012134f, -0.027632f, 0.319464f),
          vec3f(-0.584883f, -0.068089f, -0.916769f),
          vec3f(0.586628f, 0.689520f, 0.492651f),
          vec3f(0.591489f, 0.830386f, 0.451270f),
          vec3f(0.907468f, 0.058642f, -0.064133f),
          vec3f(-0.409418f, 0.649977f, 0.176513f),
          vec3f(-0.813938f, 0.377345f, -0.340292f),
          vec3f(-0.826751f, -0.686926f, -0.867467f),
          vec3f(0.132262f, -0.482955f, -0.191652f),
          vec3f(0.548982f, 0.959647f, -0.084763f),
          vec3f(0.929383f, -0.660522f, -0.326830f),
          vec3f(-0.750062f, 0.937860f, 0.983628f),
          vec3f(0.608831f, 0.467534f, 0.236503f),
          vec3f(0.668190f, -0.401794f, 0.737417f),
          vec3f(0.889422f, -0.039068f, 0.077340f),
          vec3f(-0.863822f, -0.930003f, -0.000523f),
          vec3f(0.549417f, -0.551091f, 0.244398f),
          vec3f(-0.030925f, 0.937450f, -0.307418f),
          vec3f(0.236684f, -0.876769f, 0.780809f),
          vec3f(-0.027881f, -0.991469f, -0.414041f),
          vec3f(0.379884f, -0.876627f, -0.795336f),
          vec3f(-0.536710f, -0.050901f, 0.155845f),
          vec3f(-0.148198f, -0.535733f, -0.671902f),
          vec3f(0.189763f, -0.451382f, 0.149612f),
          vec3f(0.709203f, -0.173504f, 0.947234f),
          vec3f(-0.768021f, -0.292941f, -0.330967f),
          vec3f(-0.890336f, 0.969067f, 0.621095f),
          vec3f(-0.977775f, 0.638416f, 0.824997f),
          vec3f(-0.723522f, 0.183983f, -0.551780f),
          vec3f(-0.787517f, -0.115524f, -0.246968f),
          vec3f(-0.809055f, 0.536130f, -0.895497f),
          vec3f(-0.878143f, 0.099918f, -0.015576f),
          vec3f(-0.760086f, -0.077847f, 0.220870f),
          vec3f(0.894756f, 0.236358f, 0.302472f),
          vec3f(-0.611196f, -0.846520f, -0.990076f),
          vec3f(0.233505f, 0.043593f, 0.384052f),
          vec3f(0.403472f, 0.775998f, -0.891345f),
          vec3f(0.794706f, 0.700698f, 0.661911f),
          vec3f(-0.850392f, -0.042229f, 0.722102f),
          vec3f(0.414002f, -0.747568f, 0.712154f),
          vec3f(0.844955f, 0.377268f, 0.288798f),
          vec3f(0.336126f, 0.246536f, -0.196885f),
          vec3f(-0.848532f, 0.428280f, 0.838675f),
          vec3f(-0.109196f, 0.232969f, -0.859602f),
          vec3f(0.970410f, 0.358852f, 0.880201f),
          vec3f(-0.111663f, 0.409217f, 0.630983f),
          vec3f(-0.813386f, 0.068489f, -0.589250f),
          vec3f(-0.452169f, -0.411164f, 0.051681f),
          vec3f(0.049602f, -0.841389f, -0.971137f),
          vec3f(-0.481295f, -0.737182f, -0.360160f),
          vec3f(-0.454485f, 0.528266f, -0.736493f),
          vec3f(0.795117f, -0.905093f, -0.663373f),
          vec3f(0.779881f, 0.506907f, 0.101171f),
          vec3f(0.467413f, 0.787775f, 0.564954f),
          vec3f(0.043115f, -0.309967f, 0.510537f),
          vec3f(-0.355282f, 0.534420f, -0.982729f),
          vec3f(0.094022f, 0.042513f, 0.504429f),
          vec3f(-0.530636f, -0.182763f, -0.187554f),
          vec3f(0.790452f, 0.847229f, 0.418170f),
          vec3f(0.945899f, -0.326467f, -0.933633f),
          vec3f(0.203171f, 0.470256f, 0.497053f),
          vec3f(0.461844f, 0.326525f, 0.134354f),
          vec3f(-0.223348f, -0.098939f, -0.109320f),
          vec3f(-0.460567f, -0.677495f, 0.333697f),
          vec3f(-0.600737f, 0.043106f, -0.561227f),
          vec3f(0.647314f, -0.677096f, 0.918323f),
          vec3f(-0.265728f, 0.243906f, -0.003034f),
          vec3f(-0.471182f, 0.373421f, -0.250165f),
          vec3f(-0.698640f, 0.657931f, 0.524001f),
          vec3f(0.642862f, -0.726892f, 0.600826f),
          vec3f(-0.468948f, -0.469484f, 0.325685f),
          vec3f(0.734276f, 0.083531f, -0.669931f),
          vec3f(-0.394845f, 0.577752f, 0.867494f),
          vec3f(0.949607f, -0.632238f, -0.049753f),
          vec3f(0.348220f, 0.300817f, -0.560772f),
          vec3f(0.836075f, -0.768229f, -0.082709f),
          vec3f(0.791013f, 0.657974f, -0.431943f),
          vec3f(0.218720f, 0.178011f, 0.506670f),
          vec3f(-0.476978f, -0.056195f, -0.583400f),
          vec3f(-0.286634f, 0.623660f, -0.352995f),
          vec3f(0.188879f, 0.227285f, -0.770871f),
          vec3f(-0.128660f, -0.158154f, 0.357722f),
          vec3f(-0.526599f, 0.907741f, 0.268980f),
          vec3f(-0.626612f, -0.264950f, -0.551487f),
          vec3f(-0.967854f, 0.566988f, -0.500457f),
          vec3f(0.237053f, -0.023746f, 0.066752f),
          vec3f(-0.623238f, -0.411696f, 0.936224f),
          vec3f(0.817466f, 0.015414f, 0.765661f),
          vec3f(-0.879830f, 0.700611f, -0.882927f),
          vec3f(0.292901f, 0.415531f, 0.187458f),
          vec3f(-0.997720f, 0.777951f, -0.596810f),
          vec3f(0.722185f, -0.067503f, -0.565989f),
          vec3f(-0.867843f, 0.247547f, -0.736280f),
          vec3f(0.689068f, -0.286234f, 0.977778f),
          vec3f(-0.474260f, -0.155269f, 0.205609f),
          vec3f(-0.712080f, -0.383039f, 0.228399f),
          vec3f(0.893931f, 0.052682f, 0.816261f),
          vec3f(0.509198f, -0.572662f, -0.910674f),
          vec3f(0.484740f, 0.429090f, -0.060710f),
          vec3f(0.727489f, -0.188255f, 0.028761f),
          vec3f(0.612916f, 0.664992f, 0.307938f),
          vec3f(-0.802878f, -0.463471f, -0.075567f),
          vec3f(-0.073774f, 0.998550f, -0.724746f),
          vec3f(0.183991f, -0.431659f, 0.469751f),
          vec3f(0.603272f, 0.331812f, -0.414016f),
          vec3f(0.445858f, 0.218315f, -0.367712f),
          vec3f(0.892548f, -0.728024f, -0.932576f),
          vec3f(0.271464f, 0.052237f, 0.061841f),
          vec3f(0.879242f, 0.675869f, 0.702161f),
          vec3f(0.341024f, -0.492115f, 0.313798f),
          vec3f(0.790026f, 0.600630f, -0.712133f),
          vec3f(0.661089f, 0.182058f, 0.412647f),
          vec3f(-0.524222f, 0.646743f, 0.425599f),
          vec3f(-0.474993f, -0.633925f, 0.720539f),
          vec3f(0.957676f, -0.250612f, -0.674880f),
          vec3f(0.269506f, 0.359974f, -0.254608f),
          vec3f(0.955507f, -0.451518f, -0.196128f),
          vec3f(-0.960232f, -0.795880f, -0.291050f),
          vec3f(-0.555729f, 0.128960f, -0.559386f),
          vec3f(0.568070f, 0.952077f, -0.591299f),
          vec3f(-0.506955f, -0.854154f, -0.227233f),
          vec3f(-0.651722f, 0.125073f, 0.933279f),
          vec3f(0.119090f, -0.046748f, -0.578125f),
          vec3f(-0.409764f, -0.387452f, -0.406052f),
          vec3f(-0.376893f, 0.091699f, -0.749987f),
          vec3f(0.954200f, 0.860731f, 0.059572f),
          vec3f(-0.236002f, -0.306695f, 0.443262f),
          vec3f(-0.519981f, -0.850549f, -0.287332f),
          vec3f(0.348319f, 0.590265f, 0.234273f),
          vec3f(0.257288f, -0.329304f, -0.667756f),
          vec3f(0.669262f, -0.177549f, 0.308395f),
          vec3f(-0.647283f, -0.993708f, 0.620220f),
          vec3f(0.195077f, -0.448614f, -0.072247f),
          vec3f(-0.387541f, 0.101548f, 0.141874f),
          vec3f(-0.101342f, 0.542262f, -0.715318f),
          vec3f(-0.871409f, -0.845599f, -0.519951f),
          vec3f(-0.930679f, 0.079791f, 0.037333f),
          vec3f(0.911917f, 0.473307f, 0.689331f),
          vec3f(0.185781f, 0.884831f, -0.949999f),
          vec3f(0.493641f, -0.702234f, -0.983978f),
          vec3f(0.705126f, -0.595460f, 0.382528f),
          vec3f(-0.116116f, 0.980131f, 0.622060f),
          vec3f(0.226570f, -0.662742f, -0.526367f),
          vec3f(0.725228f, 0.793838f, -0.736896f),
          vec3f(-0.856890f, 0.054692f, -0.585274f),
          vec3f(0.294275f, 0.294340f, -0.817653f),
          vec3f(-0.234902f, -0.444210f, -0.614040f),
          vec3f(-0.375795f, 0.069699f, 0.857098f),
          vec3f(-0.733102f, 0.839371f, -0.134084f),
          vec3f(0.538353f, 0.197648f, -0.152947f),
          vec3f(0.621729f, 0.550598f, 0.710832f),
          vec3f(0.893252f, 0.009951f, -0.845614f),
          vec3f(-0.072898f, 0.948685f, -0.400049f),
          vec3f(0.001330f, -0.099888f, -0.609876f),
          vec3f(-0.055420f, -0.757842f, -0.383688f),
          vec3f(-0.989151f, 0.726889f, 0.888176f),
          vec3f(-0.224738f, 0.055326f, 0.907574f),
          vec3f(0.926921f, 0.933691f, 0.843116f),
          vec3f(0.205275f, -0.075231f, 0.177762f),
          vec3f(-0.166969f, 0.179626f, -0.911754f),
          vec3f(0.515135f, -0.192318f, 0.509006f),
          vec3f(-0.668226f, 0.629990f, -0.111065f),
          vec3f(0.505666f, 0.923934f, -0.345031f),
          vec3f(0.015969f, -0.359537f, 0.149671f),
          vec3f(0.974975f, 0.550218f, -0.085417f),
          vec3f(0.305874f, -0.263738f, -0.932512f),
          vec3f(-0.220687f, 0.697764f, 0.293751f),
          vec3f(0.940138f, -0.663914f, -0.640449f),
          vec3f(-0.950280f, -0.184093f, 0.523278f),
          vec3f(0.502721f, -0.565895f, 0.910368f),
          vec3f(0.595101f, -0.502263f, -0.277325f),
          vec3f(-0.475820f, -0.490731f, 0.100023f),
          vec3f(0.213044f, 0.282338f, -0.141788f),
          vec3f(-0.999785f, 0.364417f, 0.210078f),
          vec3f(-0.896330f, -0.745713f, 0.896309f),
          vec3f(0.379615f, -0.586683f, 0.155231f),
          vec3f(0.731408f, 0.985412f, 0.063440f),
          vec3f(0.349495f, 0.911786f, -0.633517f),
          vec3f(-0.217190f, -0.035320f, -0.774343f),
          vec3f(-0.243336f, -0.404898f, -0.032249f),
          vec3f(-0.080328f, 0.003646f, 0.602229f),
          vec3f(0.463875f, 0.361834f, -0.218252f),
          vec3f(0.076559f, 0.005251f, -0.659211f),
          vec3f(0.090149f, 0.440450f, -0.181935f),
          vec3f(-0.526566f, 0.267006f, -0.142903f),
          vec3f(0.066747f, 0.235429f, 0.081329f)
      );

const radius: f32 = 0.2;
fn compute_ssao(pixel_coord: vec2<u32>) {
    let loadedNormal = textureLoad(normals, pixel_coord, 0).xyz;
    var normal = loadedNormal;

    var fragPos = vec3f(0.0, 0.0, 0.0);
    if(loadedNormal.x == 0.0 && loadedNormal.y == 0.0 && loadedNormal.z == 0.0) {
        let u_coord = (f32(pixel_coord.x) * camera.inv_screen_size.x) * 2.0 - 1.0;
        let v_coord = (f32(pixel_coord.y) * camera.inv_screen_size.y) * 2.0 - 1.0;
        let near_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 0.0, 1.0);
        let near_world = near_world1.xyz / near_world1.w;
        let far_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 1.0, 1.0);
        let far_world = far_world1.xyz / far_world1.w;

        var u = -near_world.z / (far_world.z - near_world.z);
        if u < 0.0 {
            u = 1.0 - u;
        }
        fragPos = near_world + u * (far_world - near_world);
        fragPos = (camera.view * vec4f(fragPos, 1.0)).xyz;
        normal = -normalize((camera.view_tr_inv * vec4f(0.0, 0.0, 1.0, 1.0)).xyz);
    } else {
        normal = -normalize(loadedNormal);
        fragPos = textureLoad(positions, pixel_coord, 0).xyz;
    }


    let noise_sample_coords = pixel_coord % vec2u(16, 16);
    let noise_vec = noise[noise_sample_coords.y * 16 + noise_sample_coords.x];
    let randomVec = (camera.view * vec4f(noise_vec, 0.0)).xyz;
    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < 8; i++) {
        var kl = kernel[i];
        kl.z = kl.z * 0.5 + 0.5;
        let dist_scale = f32(i) / 8.0;
        let samplePos = fragPos + (TBN * (kl * lerp(0.1, 1.0, dist_scale * dist_scale))) * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        let NdotS = max(dot(normal, viewSampleDir), 0.0);
        if(NdotS == 0.0) {
            continue;
        }

        let offset2 = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset2.xy / offset2.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv / camera.inv_screen_size);

        let sampleDepth = textureLoad(positions, screenCoord, 0).z;

        let rangeCheck = smoothstep(0.1, 1.0, (radius) / abs(fragPos.z - sampleDepth));

        occlusion += select(0.0, 1.0, sampleDepth > samplePos.z + 0.025) * rangeCheck;
    }

    occlusion = occlusion / 8.0;
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}

fn lerp(a: f32, b: f32, f:f32) -> f32 {
    return a + f * (b - a);
}


fn compute_blur(pixel_coord: vec2<u32>) {
   var result = 0.0;
    for (var y = -2; y <= 2; y++) {
      for (var x = -2; x <= 2; x++) {
        let offset = vec2i(pixel_coord) + vec2i(x, y);
        result += textureLoad(ssao_texture, offset).r;
      }
    }
    textureStore(ssao_texture, pixel_coord, vec4f(result / 25.0, 0.0, 0.0, 0.0));
}