# B559 — Aloha Beach foliage explodes in the browser (Brave)

User report 2026-10-17 (Brave): after a few games on Aloha Beach, the trees' and plants' triangles stretch across the
screen (`context/bugs/aloha-foliage-brave.png`: doubles, 1P and 3P human, Sets 0 Games 2). A second shot from the same
session (`context/bugs/aloha-shadow-brave.png`) shows the court's shade map blocky, as if sampled nearest, and an
oversized black tree shadow.

**Status: blocked, no repro.** Nothing is changed in the game.

## What the screenshot says

The leaf texture stays coherent across each stretched triangle, so the vertices drawn are real foliage vertices
(UVs and colours intact) joined in the wrong order or with wrong positions. Two things would do that: a tree's index
range holding another mesh's indices, which then point into the tree's own vertex buffer, or the right indices
read from another foliage mesh's vertex buffer. A wrong model matrix would not: it would move or scale the whole
tree intact.

## Repro attempts (all clean)

Headless chromium on the real GPU (`CHROME_FLAGS="--enable-features=Vulkan --use-angle=vulkan"`, 60 Hz; SwiftShader
only manages 10 Hz with black shots), page served from a wasm-bindgen build of this branch (`web/pkg`,
`web/pkg-webgpu`), profile with the pack imported, autoplay on, screenshots every 15 s (`context/b559/`):

| run | backend | set-up | length | result |
|---|---|---|---|---|
| run6 | WebGL2 (`backend: Gl`, `&webgl2`) | `play&stage=2&pads=-,-,-,-` bot doubles | 900 s, a full match to 4-2 and the result screen | no stretched foliage in any shot, no console errors |
| run7 | WebGPU (`backend: BrowserWebGpu`) | the same | 900 s, a full match through the stats screen | clean, no errors |
| run8 | WebGL2 | `pads=k,-,-,-`, J every 2 s | aborted: the keyboard human never served (one tap per 2 s didn't take) and the match sat paused | — |
| run9 | WebGL2 | menu flow: Enter through title → Local → character select → Ready → match | 80 s, mapped the menu (Tab = singles/doubles; the court is Random and lands on grass) | clean |

## Code read (no fault found)

- `vertex_patch.rs`: a write is checked against the mesh's own allocation and goes to the slab's buffer at
  `(range.start + first) * stride`. The costume sway (88-byte skinned vertices) can't share a slab with stage meshes
  (48 bytes: pos, normal, uv, colour), since slabs are keyed by element size. On WebGL2 every vertex array is
  its own large-object buffer (no `BASE_VERTEX`), so the sway can't reach foliage vertices there at all. Not
  bisected in the browser because the bug never showed.
- `slab_allocator.rs` (patched): `reallocate_slab` submits the old→new copy before the frame's writes land.
  `free_empty_slabs` keeps refilled slabs. A live key's old allocation is freed before it is reallocated, and a
  large-object slab gets a fresh id. One latent issue: `grow_if_necessary`'s `CantGrow` leaves `current_slot_capacity`
  bumped and leaks the offset-allocator range. It needs a slab at the 512 MiB maximum, so it can't be what happened here.
- `mesh/allocator.rs`: modified meshes are freed, then reallocated, then copied in the same `PrepareAssets` pass.
  CPU batching (WebGL2) reads the vertex and index slices at draw time, so its offsets can't be stale.
- wgpu-hal 29.0.4 gles: index-buffer copies on WebGL go element → element and rebind the current index buffer
  afterwards. Writes from CPU-side staging use `bufferSubData`. Each render pass sets its index buffer itself.
- `net_hide.rs` hides near trees by `Visibility` only, and the auto-tune steps (`graphics::cheaper`) don't touch court
  meshes. `lod.rs` is characters only.

## What would unblock it

From the reporting user: OS, GPU, Brave version, and whether the page ran WebGPU or WebGL2 (the page's
"Try WebGL2" link and Settings → Display → Renderer). Ideally also a console log (F12) from a session where it
happens. Then repro on that backend and driver.

On our side: try a Windows machine with Brave (ANGLE on D3D11, not Vulkan), and a keyboard or pad human in
doubles. To drive a human headless, `web_check.mjs` needs a held key (rawKeyDown … keyUp after ~100 ms worked
for the menus but not for the serve).

## Tooling

`tools/web_check.mjs`: visit 2 now asks for `#iso` again until the node holds. The page can replace its document
while it starts, which failed the first visit with "Could not find node with given id".

## Not verified / not 1:1

- The bug itself: never reproduced on Linux/ANGLE-Vulkan (WebGL2 or WebGPU), so there's no root cause and no fix.
  Brave on the user's OS and GPU wasn't available.
- The suggested bisect (skip the sway's `vertex_patch::send`) wasn't run: with no repro there was nothing to bisect.
- A human player's session (the report's 1P/3P) wasn't covered: the headless key taps didn't serve (run8).
