#!/usr/bin/env python3
"""Upscale every mod's textures with chaiNNer, skipping the ones already done.

    python3 modding/tools/upscale_mods.py MODEL.pth [MODS_DIR ...] [--chain tools/chainner/upscale.chn]

For each `mod.json` under MODS_DIR (default `mods/`), every image embedded in its costume `.glb`s and
`racket.glb` without a `textures/upscaled/<glb stem>/<name>.png` yet is upscaled with the chain (its folders and
model filled in here, run headless with `chainner run`) and written there. The game draws those when the
"upscaled textures" setting is on. Already-upscaled images are skipped unless their source changed
(`textures/upscaled/sources.json`); delete a PNG to redo it. Rerun after adding or rebuilding mods.

Slow (~2.5 s an image whatever its size)? chaiNNer 0.25.1 starts each node's work from a worker thread without
waking its event loop. In `resources/src/api/lazy.py` `Lazy.from_coroutine`, replace the `create_task` + polling
loop with `return asyncio.run_coroutine_threadsafe(coroutine, loop).result()` (20x faster on a GPU).
"""
import argparse, glob, hashlib, json, os, shutil, signal, struct, subprocess, sys, tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def glb_images(path):
    """(file name, PNG bytes) of each embedded image, named as the game looks them up (mods.rs `image_file`)."""
    b = path.read_bytes()
    n = struct.unpack_from('<I', b, 12)[0]
    j = json.loads(b[20:20 + n])
    blob = b[20 + n + 8:]
    for i, im in enumerate(j.get('images', [])):
        v = j['bufferViews'][im['bufferView']]
        o = v.get('byteOffset', 0)
        yield f"{im.get('name', f'image{i}')}.png", blob[o:o + v['byteLength']]


def sources(d):
    """`textures/upscaled/sources.json` of mod folder `d`: upscaled path -> SHA-1 of the image it was made from."""
    f = d / 'textures/upscaled/sources.json'
    return json.loads(f.read_text()) if f.exists() else {}


def todo(roots):
    """(mod folder, source PNG bytes, upscaled path) for every image not upscaled yet or changed since."""
    for root in roots:
        # glob, not rglob: mods/ is often a symlink to a build folder
        for mj in sorted(glob.glob(f'{root}/**/mod.json', recursive=True)):
            d = Path(mj).parent
            done = sources(d)
            glbs = [d / c for c in json.loads(Path(mj).read_text()).get('costumes', [])] + [d / 'racket.glb']
            for g in filter(Path.is_file, glbs):
                for name, png in glb_images(g):
                    out = d / 'textures/upscaled' / g.stem / name
                    if not out.exists() or done.get(f'{g.stem}/{name}') != hashlib.sha1(png).hexdigest():
                        yield d, png, out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('model', type=Path, help='the 4x upscaling model (.pth etc.)')
    ap.add_argument('roots', nargs='*', default=[REPO / 'mods'], help='folders searched for mod.json')
    ap.add_argument('--chain', type=Path, default=REPO / 'tools/chainner/upscale.chn')
    ap.add_argument('--chainner', default=str(Path.home() / '.local/bin/chainner-app/chainner'), help='the chaiNNer binary')
    ap.add_argument('--batch', type=int, default=500, help='images per chaiNNer run')
    a = ap.parse_args()

    jobs = list(todo(a.roots))
    if not jobs:
        print('nothing to upscale')
        return
    print(f'{len(jobs)} textures to upscale')
    done = 0
    # batches, so an interrupted run keeps what it finished
    for k in range(0, len(jobs), a.batch):
        done += upscale(jobs[k:k + a.batch], a)
        print(f'{done}/{len(jobs)} upscaled')
    if done < len(jobs):
        sys.exit('some textures were not written; rerun to retry them')


def upscale(jobs, a):
    """Run the chain on `jobs`, move the results into place; how many were written."""
    with tempfile.TemporaryDirectory() as tmp:
        src, dst = Path(tmp, 'in'), Path(tmp, 'out')
        src.mkdir(), dst.mkdir()
        # ponytail: flat numbered names, since the chain saves by file name only (no subfolders)
        for i, (_, png, _) in enumerate(jobs):
            (src / f'{i:06}.png').write_bytes(png)
        chain = json.loads(a.chain.read_text())
        # headless: drop preview nodes, which encode every result for a GUI that isn't there (most of the run time)
        c = chain['content']
        views = {n['id'] for n in c['nodes'] if n['data']['schemaId'] == 'chainner:image:view'}
        c['nodes'] = [n for n in c['nodes'] if n['id'] not in views]
        c['edges'] = [e for e in c['edges'] if e['target'] not in views]
        for n in c['nodes']:
            kind, data = n['data']['schemaId'], n['data']['inputData']
            if kind == 'chainner:image:load_images':
                data['0'] = str(src)
            elif kind == 'chainner:image:save':
                data['1'] = str(dst)
            elif kind == 'chainner:pytorch:load_model':
                data['0'] = str(a.model.resolve())
        run = Path(tmp, 'run.chn')
        run.write_text(json.dumps(chain))
        # --no-sandbox last: chaiNNer takes unknown options as positionals, so one before `run` eats the file
        # AMD (ROCm): MIOpen's default kernel search costs 15-20 s per new image size, in every chaiNNer run
        env = {'MIOPEN_FIND_MODE': 'FAST', **os.environ}
        p = subprocess.Popen([a.chainner, 'run', str(run), '--no-sandbox'], start_new_session=True, env=env)
        try:
            if p.wait():
                sys.exit(f'chaiNNer failed ({p.returncode})')
        finally:
            # its Python backend outlives `chainner run`: end the whole group
            try:
                os.killpg(p.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        done, made = 0, {}
        for i, (d, png, out) in enumerate(jobs):
            f = dst / f'{i:06}.png'
            if f.exists():
                out.parent.mkdir(parents=True, exist_ok=True)
                shutil.move(f, out)
                made.setdefault(d, sources(d))[f'{out.parent.name}/{out.name}'] = hashlib.sha1(png).hexdigest()
                done += 1
        for d, m in made.items():
            (d / 'textures/upscaled/sources.json').write_text(json.dumps(m, indent=0, sort_keys=True))
        return done

if __name__ == '__main__':
    main()
