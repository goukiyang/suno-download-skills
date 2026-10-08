# -*- coding: utf-8 -*-
"""Install only into an explicit directory; never log in or replace skills."""
from pathlib import Path
import argparse, json, shutil, sys
from importlib.util import spec_from_file_location, module_from_spec

root=Path(__file__).resolve().parent
sys.dont_write_bytecode=True
spec=spec_from_file_location('verify_package',root/'verify-package.py')
module=module_from_spec(spec);spec.loader.exec_module(module)

def install(dest,dry_run):
    count=module.verify(root)
    if not dest.is_absolute() or '..' in dest.parts: raise ValueError('Use a new explicit absolute skill directory')
    existing=next((p for p in [dest,*dest.parents] if p.exists()),None)
    if existing and (not existing.is_dir() or existing.is_symlink()): raise ValueError('Destination is not an actual directory')
    for ancestor in [dest,*dest.parents]:
        if ancestor.is_symlink(): raise ValueError('Destination must not contain symlinks')
    names=sorted(p.name for p in (root/'skills').iterdir() if p.is_dir())
    if any((dest/name).exists() or (dest/name).is_symlink() for name in names): raise ValueError('An included skill already exists; nothing was replaced')
    if not dry_run:
        dest.mkdir(parents=True,exist_ok=True)
        # Reserve each new directory exclusively. A race cannot overwrite an existing skill.
        reserved=[]
        for name in names:
            (dest/name).mkdir(exist_ok=False);reserved.append(name)
        for name in reserved:
            shutil.copytree(root/'skills'/name,dest/name,dirs_exist_ok=True)
    return {'status':'dry_run' if dry_run else 'installed','skills':names,'checked_files':count,'auth':False,'network':False,'overwrites':0}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--dest',type=Path,required=True);parser.add_argument('--dry-run',action='store_true')
    args=parser.parse_args()
    try: print(json.dumps(install(args.dest,args.dry_run)))
    except (ValueError,OSError) as error:
        print(json.dumps({'status':'failed','reason':str(error)}));sys.exit(1)
