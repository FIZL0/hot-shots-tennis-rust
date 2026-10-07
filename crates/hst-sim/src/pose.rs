//! Skeleton poses from the game's motions: a node's local matrix from its keys (rotation quaternion, position),
//! model-space matrices down the hierarchy. Row-vector matrices as the game's (model = local · parent).

use hst_data::ani::Anim;

pub type M4 = [[f32; 4]; 4];

/// A skeleton: node names, parents (index before child) and rest local matrices.
pub struct Skeleton {
    pub names: Vec<String>,
    pub parent: Vec<Option<usize>>,
    pub rest: Vec<M4>,
}

fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

/// Row-vector rotation of a stored key (the key is the conjugate of the column-sense local rotation, so the
/// row-vector matrix is the key's own column-sense matrix).
fn rotation([x, y, z, w]: [f32; 4]) -> M4 {
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w), 0.0],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w), 0.0],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Model-space matrix of every node at the motion's first frame.
/// ponytail: first keys only (what the turn's pelvis table needs); keyed sampling and the game's exact
/// evaluator (`0x140360`/`0x1404c0`) come with the motion playback port.
pub fn first_frame(sk: &Skeleton, anim: &Anim) -> Vec<M4> {
    let mut model: Vec<M4> = Vec::with_capacity(sk.names.len());
    for i in 0..sk.names.len() {
        let mut local = sk.rest[i];
        if let Some(t) = anim.tracks.iter().find(|t| t.name == sk.names[i]) {
            if let Some(&(_, q)) = t.rotation.first() {
                let r = rotation(q);
                for row in 0..3 {
                    local[row] = r[row];
                }
            }
            if let Some(&(_, p)) = t.position.first() {
                local[3] = [p[0], p[1], p[2], 1.0];
            }
        }
        model.push(match sk.parent[i] {
            Some(p) => mul(&local, &model[p]),
            None => local,
        });
    }
    model
}
