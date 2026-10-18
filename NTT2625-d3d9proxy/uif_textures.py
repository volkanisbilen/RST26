"""Lists the textures every UIF of uif_in\\ uses and which of them the client's UI pack does not have.
Usage: python uif_textures.py [gameDir]"""
import glob
import os
import sys

import uifrender as U
import uifview as V

game = sys.argv[1] if len(sys.argv) > 1 else r'D:\ai\JstKO Client'
t = U.Tex(game)
for f in sorted(glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'uif_in', '*.uif'))):
    texs = set()

    def walk(e):
        if e.tex:
            texs.add(os.path.basename(e.tex.replace('\\', '/')).lower())
        for c in e.children:
            walk(c)

    for top in V.load(f):
        walk(top)
    miss = sorted(n for n in texs if n not in t.pack)
    print(os.path.basename(f), 'textures', len(texs), 'missing', miss)
