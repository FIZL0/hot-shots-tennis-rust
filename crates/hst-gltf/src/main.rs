//! hst-gltf — export Hot Shots Golf Fore! (and Hot Shots Tennis) characters to glTF 2.0 binaries.
//!
//! ```text
//! hst-gltf info <file.mdl>                    node tree of a model
//! hst-gltf hst  <HST xb root> <out dir>       HST pc00 costume 0 + racket + its PCANI motions (reference rig)
//! hst-gltf fore <out/files/fore> <out root>   every Fore character × costume, and per character its motions
//! hst-gltf retarget <mdl> <mtl> <out.glb> <ani2…>  Biped motions (e.g. HST's) onto a model, names matched sans spaces
//! ```
//! A .glb holds the model's node tree as joints (game space: Y down, feet at the origin, under a `game_space`
//! node that turns it Y up), one skinned mesh with a primitive per MTL material (KHR_materials_unlit, PNG
//! textures), the face morph targets, and the motions as animations: the game's own sampler (`hst_sim::pose::Clip`:
//! squad rotations, Hermite positions scaled by bone length) baked at every game frame, `.MOR` face weights
//! likewise (`hst_sim::face`). Time is 3ds Max ticks / 4800.

use std::path::{Path, PathBuf};

use hst_data::{ani, mdl, mor, mtl};
use hst_sim::pose::{Clip, Skeleton};
use serde_json::{Value, json};

type M4 = [[f32; 4]; 4];
type R<T> = Result<T, String>;

/// Fore character names, by pcNN: SYSTEM.BIN's 0x40-byte character records at 0xfde40 (the record's first byte
/// is the body type, which matches every `pcNN_tTT` model file name).
const FORE_NAMES: [&str; 24] = [
    "Phoebe", "Mike", "Emma", "Sam", "Misaki", "Brad", "Chaos", "Regis", "Maya", "Falcon", "Renee", "Z", "Mel", "Allan",
    "Tiffany", "Kamala", "Toni", "T-Bone", "Lin", "Louise", "Zeus", "Hubert", "Ratchet", "Jak",
];

struct Motion {
    name: String,
    anim: ani::Anim,
    face: Option<mor::Tracks>,
}

/// A rigid model carried by a joint (the racket).
struct Rigid {
    joint: usize,
    model: mdl::Model,
    mtl: mtl::Mtl,
}

#[derive(Default)]
struct Glb {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Glb {
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut v = json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len()});
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(v);
        self.views.len() - 1
    }

    fn floats<const N: usize>(&mut self, data: &[[f32; N]], ty: &str, minmax: bool, target: Option<u32>) -> usize {
        let bytes: Vec<u8> = data.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.view(&bytes, target);
        let mut a = json!({"bufferView": view, "componentType": 5126, "count": data.len(), "type": ty});
        if minmax {
            let min: Vec<f32> = (0..N).map(|k| data.iter().map(|v| v[k]).fold(f32::INFINITY, f32::min)).collect();
            let max: Vec<f32> = (0..N).map(|k| data.iter().map(|v| v[k]).fold(f32::NEG_INFINITY, f32::max)).collect();
            a["min"] = json!(min);
            a["max"] = json!(max);
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }

    fn ints(&mut self, bytes: &[u8], component: u32, ty: &str, count: usize, target: Option<u32>) -> usize {
        let view = self.view(bytes, target);
        self.accessors.push(json!({"bufferView": view, "componentType": component, "count": count, "type": ty}));
        self.accessors.len() - 1
    }
}

fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

/// General 4×4 inverse (Gauss-Jordan with partial pivoting).
fn inverse(m: &M4) -> M4 {
    let mut a: [[f64; 8]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| if c < 4 { m[r][c] as f64 } else if c - 4 == r { 1.0 } else { 0.0 }));
    for c in 0..4 {
        let p = (c..4).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs())).unwrap();
        a.swap(c, p);
        let d = a[c][c];
        a[c].iter_mut().for_each(|v| *v /= d);
        for r in (0..4).filter(|&r| r != c) {
            let f = a[r][c];
            for k in 0..8 {
                a[r][k] -= f * a[c][k];
            }
        }
    }
    std::array::from_fn(|r| std::array::from_fn(|c| a[r][c + 4] as f32))
}

fn norm(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// A row-vector game matrix (translation in row 3) as glTF translation, rotation (x y z w), scale.
fn trs(m: &M4) -> ([f32; 3], [f32; 4], [f32; 3]) {
    let s = [0, 1, 2].map(|r| norm([m[r][0], m[r][1], m[r][2]]));
    // column-vector rotation: its columns are the rows of m
    let r = |i: usize, j: usize| m[j][i] / s[j];
    let tr = r(0, 0) + r(1, 1) + r(2, 2);
    let q = if tr > 0.0 {
        let w = (1.0 + tr).sqrt() * 2.0;
        [(r(2, 1) - r(1, 2)) / w, (r(0, 2) - r(2, 0)) / w, (r(1, 0) - r(0, 1)) / w, w / 4.0]
    } else if r(0, 0) > r(1, 1) && r(0, 0) > r(2, 2) {
        let w = (1.0 + r(0, 0) - r(1, 1) - r(2, 2)).sqrt() * 2.0;
        [w / 4.0, (r(0, 1) + r(1, 0)) / w, (r(0, 2) + r(2, 0)) / w, (r(2, 1) - r(1, 2)) / w]
    } else if r(1, 1) > r(2, 2) {
        let w = (1.0 + r(1, 1) - r(0, 0) - r(2, 2)).sqrt() * 2.0;
        [(r(0, 1) + r(1, 0)) / w, w / 4.0, (r(1, 2) + r(2, 1)) / w, (r(0, 2) - r(2, 0)) / w]
    } else {
        let w = (1.0 + r(2, 2) - r(0, 0) - r(1, 1)).sqrt() * 2.0;
        [(r(0, 2) + r(2, 0)) / w, (r(1, 2) + r(2, 1)) / w, w / 4.0, (r(1, 0) - r(0, 1)) / w]
    };
    ([m[3][0], m[3][1], m[3][2]], unit_q(q), s)
}

fn unit_q(q: [f32; 4]) -> [f32; 4] {
    let n = (q.iter().map(|v| v * v).sum::<f32>()).sqrt();
    if n > 0.0 { q.map(|v| v / n) } else { [0.0, 0.0, 0.0, 1.0] }
}

fn png(t: &mtl::Texture) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, t.width, t.height);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().and_then(|mut w| w.write_image_data(&t.rgba)).expect("png encode");
    out
}

/// Materials of `mats` (textures, samplers by the model's wrap modes) appended to the document's lists.
fn materials(g: &mut Glb, doc: &mut Value, model: &mdl::Model, mats: &mtl::Mtl, tag: &str) -> usize {
    let first = doc["materials"].as_array().unwrap().len();
    let mut images: Vec<Option<usize>> = vec![None; mats.textures.len()];
    for (i, m) in mats.materials.iter().enumerate() {
        let c = m.color.map(|c| c.clamp(0.0, 1.0));
        let mut pbr = json!({"baseColorFactor": c, "metallicFactor": 0.0, "roughnessFactor": 1.0});
        if let Some(t) = m.texture.filter(|&t| !mats.textures[t].rgba.is_empty()) {
            let image = *images[t].get_or_insert_with(|| {
                let view = g.view(&png(&mats.textures[t]), None);
                let list = doc["images"].as_array_mut().unwrap();
                list.push(json!({"bufferView": view, "mimeType": "image/png", "name": format!("{tag}tex{t}")}));
                list.len() - 1
            });
            let wrap = |w: u8| if w == 1 { 33071 } else { 10497 };
            let [u, v] = model.wrap.get(i).copied().unwrap_or([0, 0]);
            let samplers = doc["samplers"].as_array_mut().unwrap();
            samplers.push(json!({"magFilter": 9729, "minFilter": 9729, "wrapS": wrap(u), "wrapT": wrap(v)}));
            let sampler = samplers.len() - 1;
            let textures = doc["textures"].as_array_mut().unwrap();
            textures.push(json!({"source": image, "sampler": sampler}));
            pbr["baseColorTexture"] = json!({"index": textures.len() - 1});
        }
        let name = format!("{tag}{}", readable(&m.name, "mat", i));
        doc["materials"].as_array_mut().unwrap().push(json!({
            "name": name, "pbrMetallicRoughness": pbr, "doubleSided": true,
            "extensions": {"KHR_materials_unlit": {}},
        }));
    }
    first
}

/// A name as read, or `{kind}{i}` when it is empty or was not ASCII (Shift-JIS names decode lossily).
fn readable(name: &str, kind: &str, i: usize) -> String {
    if name.is_empty() || name.contains('\u{fffd}') { format!("{kind}{i}") } else { name.to_string() }
}

fn colors(c: [u8; 4]) -> [f32; 4] {
    c.map(|c| (c as f32 / 128.0).min(1.0))
}

fn export(model: &mdl::Model, mats: &mtl::Mtl, motions: &[Motion], rigid: &[Rigid]) -> Vec<u8> {
    let mut g = Glb::default();
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": "hst-gltf"},
        "extensionsUsed": ["KHR_materials_unlit"],
        "scene": 0, "nodes": [], "meshes": [], "materials": [], "images": [], "samplers": [], "textures": [],
    });
    let n = model.node_count;
    // nodes: 0 game_space, 1..=n joints, then meshes
    let mut nodes = vec![json!({"name": "game_space", "rotation": [0.0, 0.0, 1.0, 0.0], "children": []})];
    for i in 0..n {
        let (t, r, s) = trs(&model.node_local[i]);
        let mut node = json!({"name": readable(&model.node_names[i], "node", i), "translation": t, "rotation": r});
        if s.iter().any(|s| (s - 1.0).abs() > 1e-4) {
            node["scale"] = json!(s);
        }
        let children: Vec<usize> = (0..n).filter(|&c| model.node_parent[c] == Some(i)).map(|c| c + 1).collect();
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
        if model.node_parent[i].is_none() {
            nodes[0]["children"].as_array_mut().unwrap().push(json!(i + 1));
        }
    }

    // skinned body: a primitive per material
    let mat0 = materials(&mut g, &mut doc, model, mats, "");
    let targets = model.morph_names.len();
    #[derive(Default)]
    struct Prim {
        pos: Vec<[f32; 3]>,
        nrm: Vec<[f32; 3]>,
        uv: Vec<[f32; 2]>,
        col: Vec<[f32; 4]>,
        joints: Vec<u8>,
        weights: Vec<[f32; 4]>,
        idx: Vec<u32>,
        morph: Vec<Vec<[f32; 3]>>,
    }
    let mut prims: Vec<Prim> = (0..model.materials.len()).map(|_| Prim { morph: vec![Vec::new(); targets], ..Default::default() }).collect();
    for (material, verts, tris, morph) in model.skinned() {
        let p = &mut prims[material];
        let base = p.pos.len() as u32;
        for (t, offsets) in p.morph.iter_mut().enumerate() {
            offsets.extend(morph.get(t).cloned().unwrap_or_else(|| vec![[0.0; 3]; verts.len()]));
        }
        for v in &verts {
            p.pos.push(v.pos);
            let s: [f32; 3] = std::array::from_fn(|k| v.normals[0][k] + v.normals[1][k]);
            let l = norm(s);
            p.nrm.push(if l > 1e-6 { s.map(|x| x / l) } else { [0.0, 0.0, 1.0] });
            p.uv.push(v.uv);
            p.col.push(colors(v.color));
            // merge repeated joints (two entries on one bone)
            let mut jw: Vec<(u16, f32)> = Vec::new();
            for k in 0..4 {
                if v.weights[k] > 0.0 {
                    match jw.iter_mut().find(|e| e.0 == v.joints[k]) {
                        Some(e) => e.1 += v.weights[k],
                        None => jw.push((v.joints[k], v.weights[k])),
                    }
                }
            }
            if jw.is_empty() {
                jw.push((v.joints[0], 1.0));
            }
            let total: f32 = jw.iter().map(|e| e.1).sum();
            let mut w = [0.0; 4];
            for (k, e) in jw.iter().enumerate() {
                p.joints.extend(e.0.to_le_bytes());
                w[k] = e.1 / total;
            }
            for _ in jw.len()..4 {
                p.joints.extend([0, 0]);
            }
            p.weights.push(w);
        }
        p.idx.extend(tris.iter().flatten().map(|i| i + base));
    }
    let mut primitives = Vec::new();
    let any_morph = prims.iter().any(|p| p.morph.iter().flatten().any(|d| *d != [0.0; 3]));
    for (m, p) in prims.iter().enumerate() {
        if p.idx.is_empty() {
            continue;
        }
        let count = p.pos.len();
        let mut attrs = json!({
            "POSITION": g.floats(&p.pos, "VEC3", true, Some(34962)),
            "NORMAL": g.floats(&p.nrm, "VEC3", false, Some(34962)),
            "TEXCOORD_0": g.floats(&p.uv, "VEC2", false, Some(34962)),
            "COLOR_0": g.floats(&p.col, "VEC4", false, Some(34962)),
            "WEIGHTS_0": g.floats(&p.weights, "VEC4", false, Some(34962)),
        });
        attrs["JOINTS_0"] = json!(g.ints(&p.joints, 5123, "VEC4", count, Some(34962)));
        let idx: Vec<u8> = p.idx.iter().flat_map(|i| i.to_le_bytes()).collect();
        let indices = g.ints(&idx, 5125, "SCALAR", p.idx.len(), Some(34963));
        let mut prim = json!({"attributes": attrs, "indices": indices, "material": mat0 + m, "mode": 4});
        if any_morph {
            let t: Vec<Value> = p.morph.iter().map(|o| json!({"POSITION": g.floats(o, "VEC3", true, Some(34962))})).collect();
            prim["targets"] = json!(t);
        }
        primitives.push(prim);
    }
    let mut mesh = json!({"name": "body", "primitives": primitives});
    if any_morph {
        mesh["weights"] = json!(vec![0.0; targets]);
        mesh["extras"] = json!({"targetNames": model.morph_names});
    }
    doc["meshes"].as_array_mut().unwrap().push(mesh);
    let ibm: Vec<[f32; 16]> = (0..n)
        .map(|i| {
            let mut m = inverse(&model.node_bind[i]);
            for r in 0..3 {
                m[r][3] = 0.0;
            }
            m[3][3] = 1.0;
            std::array::from_fn(|k| m[k / 4][k % 4])
        })
        .collect();
    let ibm = g.floats(&ibm, "MAT4", false, None);
    doc["skins"] = json!([{"inverseBindMatrices": ibm, "joints": (1..=n).collect::<Vec<_>>(), "name": "skeleton"}]);
    let body = nodes.len();
    nodes.push(json!({"name": "body", "mesh": 0, "skin": 0}));

    // rigid parts carried by a joint
    for r in rigid {
        let m0 = materials(&mut g, &mut doc, &r.model, &r.mtl, "racket_");
        let mut primitives = Vec::new();
        for (mi, packets) in r.model.materials.iter().enumerate() {
            let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
            for pk in packets {
                let base = pos.len() as u32;
                for v in &pk.vertices {
                    pos.push(v.pos);
                    let l = norm(v.normal);
                    nrm.push(if l > 1e-6 { v.normal.map(|x| x / l) } else { [0.0, 0.0, 1.0] });
                    uv.push(v.uv);
                    col.push(colors(v.color));
                }
                idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
            }
            if idx.is_empty() {
                continue;
            }
            let attrs = json!({
                "POSITION": g.floats(&pos, "VEC3", true, Some(34962)),
                "NORMAL": g.floats(&nrm, "VEC3", false, Some(34962)),
                "TEXCOORD_0": g.floats(&uv, "VEC2", false, Some(34962)),
                "COLOR_0": g.floats(&col, "VEC4", false, Some(34962)),
            });
            let ib: Vec<u8> = idx.iter().flat_map(|i: &u32| i.to_le_bytes()).collect();
            let indices = g.ints(&ib, 5125, "SCALAR", idx.len(), Some(34963));
            primitives.push(json!({"attributes": attrs, "indices": indices, "material": m0 + mi, "mode": 4}));
        }
        let meshes = doc["meshes"].as_array_mut().unwrap();
        meshes.push(json!({"name": "racket", "primitives": primitives}));
        let mesh = meshes.len() - 1;
        nodes.push(json!({"name": "racket", "mesh": mesh}));
        let me = nodes.len() - 1;
        let joint = &mut nodes[r.joint + 1];
        if joint.get("children").is_none() {
            joint["children"] = json!([]);
        }
        joint["children"].as_array_mut().unwrap().push(json!(me));
    }

    // motions
    let skeleton = Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
    let mut animations = Vec::new();
    for mo in motions {
        let clip = Clip::new(&skeleton, &mo.anim);
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        let tpf = clip.ticks_per_frame.max(1.0);
        let frames = clip.length.max(0.0).ceil() as usize + 1;
        let times: Vec<[f32; 1]> = (0..frames).map(|f| [f as f32 * tpf / 4800.0]).collect();
        let input = g.floats(&times, "SCALAR", true, None);
        for k in 0..clip.tracks.len() {
            let node = clip.tracks[k].node + 1;
            let s: Vec<_> = (0..frames).map(|f| clip.sample(k, f as f32)).collect();
            if s[0].0.is_some() {
                let mut prev = [0.0, 0.0, 0.0, 1.0];
                let q: Vec<[f32; 4]> = s
                    .iter()
                    .map(|(r, _)| {
                        let [x, y, z, w] = r.unwrap();
                        let mut q = unit_q([-x, -y, -z, w]);
                        // keep consecutive keys in one hemisphere so linear interpolation takes the short way
                        if q.iter().zip(&prev).map(|(a, b)| a * b).sum::<f32>() < 0.0 {
                            q = q.map(|v| -v);
                        }
                        prev = q;
                        q
                    })
                    .collect();
                let out = g.floats(&q, "VEC4", false, None);
                samplers.push(json!({"input": input, "output": out, "interpolation": "LINEAR"}));
                channels.push(json!({"sampler": samplers.len() - 1, "target": {"node": node, "path": "rotation"}}));
            }
            if s[0].1.is_some() {
                let p: Vec<[f32; 3]> = s.iter().map(|(_, p)| p.unwrap()).collect();
                let out = g.floats(&p, "VEC3", false, None);
                samplers.push(json!({"input": input, "output": out, "interpolation": "LINEAR"}));
                channels.push(json!({"sampler": samplers.len() - 1, "target": {"node": node, "path": "translation"}}));
            }
        }
        if let (Some(face), true) = (&mo.face, any_morph) {
            let tracks: Vec<(usize, &mor::Track)> =
                face.tracks.iter().filter_map(|t| Some((model.morph_names.iter().position(|n| *n == t.name)?, t))).collect();
            if !tracks.is_empty() {
                let ftpf = face.ticks_per_frame.max(1) as f32;
                let len = hst_sim::face::length(tracks.iter().map(|t| &t.1.ticks[..]), face.ticks_per_frame);
                let frames = len.max(0.0).ceil() as usize + 1;
                let times: Vec<[f32; 1]> = (0..frames).map(|f| [f as f32 * ftpf / 4800.0]).collect();
                let finput = g.floats(&times, "SCALAR", true, None);
                let mut w = vec![[0.0f32; 1]; frames * targets];
                for (target, t) in &tracks {
                    let mut cursor = 0;
                    for f in 0..frames {
                        w[f * targets + target][0] = hst_sim::face::sample(&t.ticks, &t.values, f as f32 * ftpf, &mut cursor, false)[0];
                    }
                }
                let out = g.floats(&w, "SCALAR", false, None);
                samplers.push(json!({"input": finput, "output": out, "interpolation": "LINEAR"}));
                channels.push(json!({"sampler": samplers.len() - 1, "target": {"node": body, "path": "weights"}}));
            }
        }
        if !channels.is_empty() {
            animations.push(json!({"name": mo.name, "samplers": samplers, "channels": channels}));
        }
    }
    if !animations.is_empty() {
        doc["animations"] = json!(animations);
    }
    doc["nodes"] = json!(nodes);
    doc["scenes"] = json!([{"nodes": [0, body]}]);
    for k in ["images", "samplers", "textures"] {
        if doc[k].as_array().unwrap().is_empty() {
            doc.as_object_mut().unwrap().remove(k);
        }
    }
    doc["accessors"] = json!(g.accessors);
    doc["bufferViews"] = json!(g.views);
    while g.bin.len() % 4 != 0 {
        g.bin.push(0);
    }
    doc["buffers"] = json!([{"byteLength": g.bin.len()}]);

    let mut js = serde_json::to_vec(&doc).unwrap();
    while js.len() % 4 != 0 {
        js.push(b' ');
    }
    let mut out = Vec::with_capacity(28 + js.len() + g.bin.len());
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend(((28 + js.len() + g.bin.len()) as u32).to_le_bytes());
    out.extend((js.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(js);
    out.extend((g.bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(g.bin);
    out
}

// ---- files

fn read(p: &Path) -> R<Vec<u8>> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// Every file under `dir`, sorted.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn ext_is(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// The sibling of `p` with extension `ext`, any case.
fn sibling(p: &Path, ext: &str) -> Option<PathBuf> {
    let stem = p.file_stem()?.to_str()?.to_ascii_lowercase();
    std::fs::read_dir(p.parent()?).ok()?.flatten().map(|e| e.path()).find(|q| {
        ext_is(q, ext) && q.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(&stem))
    })
}

fn load_model(mdl_path: &Path, mtl_path: &Path) -> R<(mdl::Model, mtl::Mtl)> {
    let model = mdl::parse(&read(mdl_path)?).map_err(|e| format!("{}: {}", mdl_path.display(), e.0))?;
    let mti = sibling(mtl_path, "mti").map(|p| read(&p)).transpose()?;
    let mats = mtl::parse(&read(mtl_path)?, mti.as_deref()).map_err(|e| format!("{}: {}", mtl_path.display(), e.0))?;
    Ok((model, mats))
}

/// Motions from every `ext` file under `dirs` (name = file stem, lower case; first of a name wins), each with
/// its sibling `.MOR`.
fn motions(files: &[PathBuf], ext: &str, out: &mut Vec<Motion>, sources: &mut Vec<String>) {
    for p in files.iter().filter(|p| ext_is(p, ext)) {
        let name = p.file_stem().unwrap().to_string_lossy().to_ascii_lowercase();
        if out.iter().any(|m| m.name == name) {
            continue;
        }
        let anim = match read(p).and_then(|d| ani::parse(&d).map_err(|e| e.0)) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("skip {}: {e}", p.display());
                continue;
            }
        };
        let face = sibling(p, "mor").and_then(|m| mor::parse(&std::fs::read(m).ok()?, 1).ok());
        sources.push(p.display().to_string());
        out.push(Motion { name, anim, face });
    }
}

fn write(path: &Path, glb: &[u8]) -> R<()> {
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, glb).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{} ({} KiB)", path.display(), glb.len() / 1024);
    Ok(())
}

fn info(path: &Path) -> R<()> {
    let m = mdl::parse(&read(path)?).map_err(|e| e.0)?;
    println!("{} nodes, {} materials, morphs {:?}", m.node_count, m.materials.len(), m.morph_names);
    for i in 0..m.node_count {
        let (t, _, s) = trs(&m.node_local[i]);
        // bind = local · parent bind (row vectors)?
        let chain = match m.node_parent[i] {
            Some(p) => mul(&m.node_local[i], &m.node_bind[p]),
            None => m.node_local[i],
        };
        let err = (0..4).flat_map(|r| (0..4).map(move |c| (r, c))).map(|(r, c)| (chain[r][c] - m.node_bind[i][r][c]).abs()).fold(0.0, f32::max);
        let parent = m.node_parent[i].map_or("-".into(), |p| p.to_string());
        let bind = m.node_bind[i][3];
        println!(
            "{i:3} {:<20} parent {parent:>3} t [{:8.3} {:8.3} {:8.3}] len {:7.3} scale [{:.3} {:.3} {:.3}] bind-chain err {err:.4} model [{:7.2} {:7.2} {:7.2}]",
            m.node_names[i], t[0], t[1], t[2], norm(t), s[0], s[1], s[2], bind[0], bind[1], bind[2]
        );
    }
    Ok(())
}

fn hst(root: &Path, out: &Path) -> R<()> {
    // every character in costume 0, with its racket and motions
    for n in 0..14 {
        let files = walk(&root.join(format!("PC/PC{n:02}C00.XB")));
        let body = format!("pc{n:02}_t");
        let mdl = files.iter().find(|p| ext_is(p, "mdl") && p.file_name().unwrap().to_string_lossy().to_ascii_lowercase().starts_with(&body)).ok_or(format!("pc{n:02}: no body"))?;
        let rk = files.iter().find(|p| ext_is(p, "mdl") && p.file_stem().unwrap().to_string_lossy().eq_ignore_ascii_case(&format!("racket_{n:02}"))).ok_or(format!("pc{n:02}: no racket"))?;
        let (model, mats) = load_model(mdl, &sibling(mdl, "mtl").ok_or("no body MTL")?)?;
        let (rmodel, rmtl) = load_model(rk, &sibling(rk, "mtl").ok_or("no racket MTL")?)?;
        let joint = model.node_names.iter().position(|n| n == "Racket").ok_or("no Racket joint")?;
        let mut list = Vec::new();
        let files = walk(&root.join(format!("PCANI/PC{n:02}ANI.XB")));
        // in the game's motion order
        for (id, _) in ani::MOTIONS.iter().enumerate() {
            let stem = ani::motion_name(id, n).unwrap().to_ascii_lowercase();
            let f: Vec<PathBuf> = files.iter().filter(|p| p.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&stem))).cloned().collect();
            motions(&f, "ani2", &mut list, &mut Vec::new());
        }
        // ball paths and root dummies are not skeletal
        list.retain(|m| !m.name.ends_with("_dummy") && !m.name.ends_with("_ball"));
        let glb = export(&model, &mats, &list, &[Rigid { joint, model: rmodel, mtl: rmtl }]);
        write(&out.join(format!("pc{n:02}_c00.glb")), &glb)?;
    }
    Ok(())
}

fn fore(root: &Path, out: &Path) -> R<()> {
    for (n, name) in FORE_NAMES.iter().enumerate() {
        let slug = name.to_ascii_lowercase().replace('-', "");
        let pc = root.join(format!("PC/PC{n:02}"));
        for c in 0..8 {
            let files = walk(&pc.join(format!("PC{n:02}{c:02}.XB")));
            let mdl = files.iter().find(|p| ext_is(p, "mdl")).ok_or(format!("pc{n:02} c{c:02}: no mdl"))?;
            let mtl = files.iter().find(|p| ext_is(p, "mtl")).ok_or(format!("pc{n:02} c{c:02}: no mtl"))?;
            let (model, mats) = load_model(mdl, mtl)?;
            write(&out.join(format!("models/fore/pc{n:02}/pc{n:02}_{slug}_c{c:02}.glb")), &export(&model, &mats, &[], &[]))?;
        }
        // motions per body variant: costumes 0–3 share one model (01p), 4–7 another (05p)
        for v in 0..2 {
            let c = v * 4;
            let files = walk(&pc.join(format!("PC{n:02}{c:02}.XB")));
            let mdl = files.iter().find(|p| ext_is(p, "mdl")).unwrap();
            let mtl = files.iter().find(|p| ext_is(p, "mtl")).unwrap();
            let (model, mats) = load_model(mdl, mtl)?;
            let mut list = Vec::new();
            let mut sources = Vec::new();
            let mut sets = vec![walk(&pc.join(format!("PC{n:02}SW{c:02}.XB")))];
            for k in 0..8 {
                sets.push(walk(&pc.join(format!("PC{n:02}{c:02}{k:02}.XB"))));
            }
            sets.push(walk(&root.join(format!("RESULT/PC{n:02}/COL{c:02}.XB"))));
            sets.push(walk(&root.join(format!("MENU/M_CHAR/COL{c}/MC{n:02}.XB"))));
            for s in &sets {
                motions(s, "ani", &mut list, &mut sources);
            }
            let path = out.join(format!("anims/fore/pc{n:02}_{slug}_{}.glb", ["c00-03", "c04-07"][v]));
            write(&path, &export(&model, &mats, &list, &[]))?;
            let names: Vec<&str> = list.iter().map(|m| m.name.as_str()).collect();
            println!("  {} motions: {}", list.len(), names.join(" "));
        }
    }
    Ok(())
}

/// HST (or any Biped) motions onto a model whose node names differ only by spaces (`Bip01LThigh` ↔ `Bip01 L Thigh`).
fn retarget(mdl_path: &Path, mtl_path: &Path, out: &Path, anis: &[String]) -> R<()> {
    let (model, mats) = load_model(mdl_path, mtl_path)?;
    let mut list = Vec::new();
    motions(&anis.iter().map(PathBuf::from).collect::<Vec<_>>(), "ani2", &mut list, &mut Vec::new());
    for m in &mut list {
        for t in &mut m.anim.tracks {
            if let Some(n) = model.node_names.iter().find(|n| n.replace(' ', "") == t.name.replace(' ', "")) {
                t.name = n.clone();
            }
        }
    }
    write(out, &export(&model, &mats, &list, &[]))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("info") if a.len() == 3 => info(Path::new(&a[2])),
        Some("hst") if a.len() == 4 => hst(Path::new(&a[2]), Path::new(&a[3])),
        Some("fore") if a.len() == 4 => fore(Path::new(&a[2]), Path::new(&a[3])),
        Some("retarget") if a.len() >= 6 => retarget(Path::new(&a[2]), Path::new(&a[3]), Path::new(&a[4]), &a[5..]),
        _ => Err("usage: hst-gltf info <mdl> | hst <xb root> <out dir> | fore <files/fore> <out root> | retarget <mdl> <mtl> <out.glb> <ani2…>".into()),
    };
    if let Err(e) = r {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
