# -*- coding: utf-8 -*-
"""Read-only package validation; no authentication or network."""
from pathlib import Path, PurePosixPath
import hashlib, json, sys

def verify(root):
    root=Path(root)
    manifest=root/'SHA256SUMS'
    expected={}
    for line in manifest.read_text().splitlines():
        checksum, name=line.split('  ',1)
        parts=PurePosixPath(name).parts
        if len(checksum)!=64 or any(c not in '0123456789abcdef' for c in checksum) or name.startswith('/') or '\\' in name or '..' in parts or ':' in name or name in expected:
            raise ValueError('Unsafe or duplicate manifest entry')
        expected[name]=checksum
    files={}
    for p in root.rglob('*'):
        # A cloned repository adds Git's own metadata, never package payload.
        if p.relative_to(root).parts[0]=='.git': continue
        if p.is_symlink(): raise ValueError('Symlinks are not allowed')
        if p.is_file() and p!=manifest:
            files[p.relative_to(root).as_posix()]=hashlib.sha256(p.read_bytes()).hexdigest()
    if files!=expected: raise ValueError('Package content differs from SHA256SUMS')
    return len(files)

if __name__=='__main__':
    try: print(json.dumps({'status':'verified','files':verify(Path(__file__).resolve().parent),'network':False,'auth':False}))
    except (ValueError,OSError) as error:
        print(json.dumps({'status':'failed','reason':str(error)}));sys.exit(1)
