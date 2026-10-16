#!/usr/bin/env python3
"""Upscale every mod's textures with chaiNNer, skipping the ones already done.

    python3 tools/upscale_mods.py MODEL.pth [MODS_DIR ...] [--chain tools/chainner/upscale.chn]
    python3 tools/upscale_mods.py --check [MODS_DIR ...]   # list what's missing or stale, per mod; exit 1 if any
    python3 tools/upscale_mods.py --prune [MODS_DIR ...]   # delete upscales no `.glb` image names any more

For each `mod.json` under MODS_DIR (default `mods/`), every image embedded in its costume `.glb`s and
`racket.glb` (a court mod: `court`/`gimmick`/`sky.glb` and its `npcs.json` people; an umpire mod: `umpire.glb`) without a `textures/upscaled/<glb stem>/<name>.png` yet is upscaled with the chain (its folders and
model filled in here, run headless with `chainner run`) and written there. The game draws those when the
"upscaled textures" setting is on. Mods with `"upscale": false` in mod.json are skipped. Already-upscaled images are skipped unless their source changed
(`textures/upscaled/sources.json`); delete a PNG to redo it. Rerun after adding or rebuilding mods.

Slow (~2.5 s an image whatever its size)? chaiNNer 0.25.1 starts each node's work from a worker thread without
waking its event loop. In `resources/src/api/lazy.py` `Lazy.from_coroutine`, replace the `create_task` + polling
loop with `return asyncio.run_coroutine_threadsafe(coroutine, loop).result()` (20x faster on a GPU).
"""
import argparse, glob, hashlib, json, os, shutil, signal, struct, subprocess, sys, tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


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


# the fixed `.glb`s a court or umpire mod draws (court_mod.rs, mods/umpire.rs)
KIND_GLBS = {'court': ['court.glb', 'gimmick.glb', 'sky.glb'], 'umpire': ['umpire.glb']}


def npc_glbs(d):
    """A court mod's people models (`npcs.json` `models`: a path, or `{"glb": path}`), in its folder."""
    f = d / 'npcs.json'
    if not f.exists():
        return []
    models = json.loads(f.read_text()).get('models', {}).values()
    return [d / (v if isinstance(v, str) else v['glb']) for v in models]


def sources(d):
    """`textures/upscaled/sources.json` of mod folder `d`: upscaled path -> SHA-1 of the image it was made from."""
    f = d / 'textures/upscaled/sources.json'
    return json.loads(f.read_text()) if f.exists() else {}


def mods(roots):
    """(mod folder, its `.glb`s that exist) of every mod that wants upscales."""
    seen = set()
    for root in roots:
        # glob, not rglob: mods/ is often a symlink to a build folder
        for mj in sorted(glob.glob(f'{root}/**/mod.json', recursive=True)):
            d = Path(mj).parent
            # each mod once, however many symlinked paths reach it (a link back up loops ~40 deep)
            if d.resolve() in seen:
                continue
            seen.add(d.resolve())
            m = json.loads(Path(mj).read_text())
            if m.get('upscale', True) is False:
                continue
            glbs = [d / c for c in m.get('costumes', [])] + [d / 'racket.glb'] + [d / f for f in KIND_GLBS.get(m.get('kind'), [])]
            if m.get('kind') == 'court':
                glbs += npc_glbs(d)
            yield d, [g for g in glbs if g.is_file()]


def todo(roots):
    """(mod folder, source PNG bytes, upscaled path) for every image not upscaled yet or changed since."""
    for d, glbs in mods(roots):
        done = sources(d)
        for g in glbs:
            for name, png in glb_images(g):
                out = d / 'textures/upscaled' / g.stem / name
                if not out.exists() or done.get(f'{g.stem}/{name}') != hashlib.sha1(png).hexdigest():
                    yield d, png, out


def unused(roots):
    """(mod folder, upscaled path) of every upscale no image of the mod's `.glb`s names any more (a rebuilt or
    dropped model's): the game never draws it, but it still ships in the mod's zip."""
    for d, glbs in mods(roots):
        want = {f'{g.stem}/{name}' for g in glbs for name, _ in glb_images(g)}
        for f in sorted((d / 'textures/upscaled').glob('*/*.png')):
            if f'{f.parent.name}/{f.name}' not in want:
                yield d, f


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('model', type=Path, nargs='?', help='the 4x upscaling model (.pth etc.)')
    ap.add_argument('roots', nargs='*', default=[REPO / 'mods'], help='folders searched for mod.json')
    ap.add_argument('--chain', type=Path, default=REPO / 'tools/chainner/upscale.chn')
    ap.add_argument('--chainner', default=str(Path.home() / '.local/bin/chainner-app/chainner'), help='the chaiNNer binary')
    ap.add_argument('--batch', type=int, default=500, help='images per chaiNNer run')
    ap.add_argument('--check', nargs='*', metavar='MODS_DIR', help="only list what's missing or stale (default mods/)")
    ap.add_argument('--prune', nargs='*', metavar='MODS_DIR', help='delete unused upscales (default mods/)')
    a = ap.parse_args()
    if a.check is not None:
        return check(a.check or [REPO / 'mods'])
    if a.prune is not None:
        return prune(a.prune or [REPO / 'mods'])
    if a.model is None:
        ap.error('the model is required unless --check')

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


def check(roots):
    """Print each mod's missing or out-of-date upscales; exit 1 if there are any."""
    per = {}
    for d, _, out in todo(roots):
        per.setdefault(d, []).append(out)
    for d, outs in per.items():
        print(f'{d.name}: {len(outs)}')
        for o in outs:
            print(f"  {o.relative_to(d)}{'' if o.exists() else ' (missing)'}")
    print(f'{sum(map(len, per.values()))} textures in {len(per)} mods to upscale')
    left = {}
    for d, f in unused(roots):
        left[d.name] = left.get(d.name, 0) + 1
    if left:
        print(f"{sum(left.values())} unused upscales in {len(left)} mods (--prune deletes them): "
              + ', '.join(f'{k} {v}' for k, v in left.items()))
    if per:
        sys.exit(1)


def prune(roots):
    """Delete unused upscales and their `sources.json` entries."""
    gone = {}
    for d, f in list(unused(roots)):
        f.unlink()
        gone.setdefault(d, []).append(f'{f.parent.name}/{f.name}')
    for d, keys in gone.items():
        m = {k: v for k, v in sources(d).items() if k not in keys}
        (d / 'textures/upscaled/sources.json').write_text(json.dumps(m, indent=0, sort_keys=True))
        for sub in (d / 'textures/upscaled').iterdir():
            if sub.is_dir() and not any(sub.iterdir()):
                sub.rmdir()
    print(f'{sum(map(len, gone.values()))} unused upscales deleted in {len(gone)} mods')


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
