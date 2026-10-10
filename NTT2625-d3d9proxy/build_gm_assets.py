"""GM panel skin (gm_panel.cpp) = cyber001 CyberACS GM tools window (CyberACS\\Dat\\DAT-19.uif, 1068 format) as is:
frame + 7 tab pages composed from the uif's own images, button state sprites (Reserved 0..3 = normal/down/on/disable),
strings / edits / buttons with their uif rects, fonts and colours. Labels of slots our server uses differently are
renamed here (RELABEL); the role list of the home page becomes 28 quick-action rows.
Textures: cyber's own (CyberACS\\UI, then the cyber client's UI folder).
Output: gm_ui\\*.pus (client: HopeGuard\\gm_ui) + gm_layout.h
Usage: python build_gm_assets.py [cyberClientDir]"""
import os, sys, struct, hashlib
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
CY = sys.argv[1] if len(sys.argv) > 1 else r'D:\KO PVP\cyber001\CyberAcs Client\Client 2383'
UIF = os.path.join(CY, 'CyberACS', 'Dat', 'DAT-19.uif')
TEXDIRS = [os.path.join(CY, 'CyberACS', 'UI'), os.path.join(CY, 'UI')]
OUT = os.path.join(HERE, 'gm_ui')
os.makedirs(OUT, exist_ok=True)

# ------------------------------------------------------------------ uif reader (1068 format)
TYPES = {0: 'BASE', 1: 'BUTTON', 2: 'STATIC', 3: 'PROGRESS', 4: 'IMAGE', 5: 'SCROLLBAR', 6: 'STRING',
         7: 'TRACKBAR', 8: 'EDIT', 9: 'AREA', 10: 'TOOLTIP', 11: 'ICON', 12: 'CHECK', 13: 'T13', 14: 'LIST'}
def read_uif(path):
    d = open(path, 'rb').read(); st = {'o': 0}
    def I():
        v = struct.unpack_from('<i', d, st['o'])[0]; st['o'] += 4; return v
    def U():
        v = struct.unpack_from('<I', d, st['o'])[0]; st['o'] += 4; return v
    def F():
        v = struct.unpack_from('<f', d, st['o'])[0]; st['o'] += 4; return v
    def S():
        n = I(); assert 0 <= n < 100000, 'bad string at %d' % st['o']
        v = d[st['o']:st['o'] + n].decode('cp1252', 'replace'); st['o'] += n; return v
    def elem(typ):
        e = {'type': TYPES.get(typ, str(typ)), 'name': S()}
        e['children'] = [elem(I()) for _ in range(I())]
        e['id'] = S(); e['rect'] = [I() for _ in range(4)]; [I() for _ in range(4)]; e['style'] = U(); e['res'] = U(); S(); S(); S()
        t = e['type']
        if t == 'IMAGE': e['tex'] = S(); e['uv'] = [F() for _ in range(4)]; F()
        elif t == 'STRING': e['font'] = S(); e['fontH'] = I(); U(); e['color'] = U(); e['text'] = S()
        elif t == 'BUTTON': [I() for _ in range(4)]; S(); S()
        elif t == 'STATIC': S()
        elif t == 'PROGRESS': F(); I(); I()
        elif t == 'AREA': I()
        elif t == 'EDIT': I(); I()
        elif t == 'LIST': S(); [I() for _ in range(4)]
        return e
    S(); n = I()
    return [elem(I()) for _ in range(n)]

# ------------------------------------------------------------------ textures (NTF, unencrypted v3 + v7 RC4)
PASS = b'owsd9012%$1as!wpow1033b%!@%12'
KEY = hashlib.sha1(PASS).digest()[:16]
def rc4(key, n):
    S = list(range(256)); j = 0
    for i in range(256):
        j = (j + S[i] + key[i % len(key)]) & 255; S[i], S[j] = S[j], S[i]
    out = bytearray(n); i = j = 0
    for k in range(n):
        i = (i + 1) & 255; j = (j + S[i]) & 255; S[i], S[j] = S[j], S[i]
        out[k] = S[(S[i] + S[j]) & 255]
    return bytes(out)
_tex = {}
def load_tex(name):
    base = os.path.basename(name.replace('\\', '/')).lower()
    if base in _tex: return _tex[base]
    for dd in TEXDIRS:
        p = os.path.join(dd, base)
        if os.path.exists(p): b = open(p, 'rb').read(); break
    else: raise FileNotFoundError(name)
    nl = struct.unpack_from('<i', b, 0)[0]; o = 4 + nl
    assert b[o:o + 3] == b'NTF', name
    ver = b[o + 3]; w, h, fmt, mip = struct.unpack_from('<IIII', b, o + 4); o += 20
    if fmt in (21, 22): pitch, rows = w * 4, h
    elif fmt in (25, 26): pitch, rows = w * 2, h
    else: bs = 8 if fmt == 0x31545844 else 16; pitch, rows = (w // 4) * bs, h // 4
    data = bytearray(b[o:o + pitch * rows])
    if ver == 7 and fmt in (0x31545844, 0x33545844, 0x35545844):
        data = bytearray(a ^ c for a, c in zip(data, rc4(KEY, len(data))))
    elif ver == 7:
        ks = rc4(KEY, pitch)
        for y in range(rows): data[y * pitch:(y + 1) * pitch] = bytes(a ^ c for a, c in zip(data[y * pitch:(y + 1) * pitch], ks))
    if fmt == 21: img = Image.frombytes('RGBA', (w, h), bytes(data), 'raw', 'BGRA')
    elif fmt == 22: img = Image.frombytes('RGBX', (w, h), bytes(data), 'raw', 'BGRX').convert('RGBA')
    elif fmt in (25, 26):
        px = bytearray(w * h * 4)
        for k, v in enumerate(struct.unpack_from('<%dH' % (w * h), data)):
            if fmt == 25: r_, g, b_, a = (v >> 10 & 31) * 255 // 31, (v >> 5 & 31) * 255 // 31, (v & 31) * 255 // 31, 255 if v & 0x8000 else 0
            else: a, r_, g, b_ = (v >> 12 & 15) * 17, (v >> 8 & 15) * 17, (v >> 4 & 15) * 17, (v & 15) * 17
            px[k * 4:k * 4 + 4] = bytes((r_, g, b_, a))
        img = Image.frombytes('RGBA', (w, h), bytes(px))
    else: img = Image.frombytes('RGBA', (w, h), bytes(data), 'bcn', {0x31545844: 1, 0x33545844: 2, 0x35545844: 3}[fmt])
    _tex[base] = img
    return img
def piece(e):
    img = load_tex(e['tex']); W, H = img.size; u0, v0, u1, v1 = e['uv']; l, t, r, b = e['rect']
    p = img.crop((round(u0 * W), round(v0 * H), round(u1 * W), round(v1 * H)))
    return p.resize((r - l, b - t), Image.BILINEAR) if p.size != (r - l, b - t) else p

def save_pus(img, name):
    img = img.convert('RGBA'); w, h = img.size
    px = bytearray()
    for r_, g, b, a in img.getdata(): px += bytes((b * a // 255, g * a // 255, r_ * a // 255, a))
    open(os.path.join(OUT, name + '.pus'), 'wb').write(b'PUSI' + struct.pack('<II', w, h) + px)

# ------------------------------------------------------------------ relabels (our server's commands, see gm_panel.cpp)
PAGES = ['home', 'user_editor', 'start_event', 'reload_tables', 'start_lottery', 'gm_commands', 'start_bots']
BTN_LABEL = {   # (page, id) -> label
    ('home', 'btn_beuser'): 'GM / Player', ('home', 'btn_unvisible'): 'Visible / Hidden',
    ('user_editor', 'btn_apply'): 'APPLY',('user_editor', 'btn_ban'): 'Ban', ('user_editor', 'btn_ban2'): 'Ban Days',
    ('user_editor', 'SHERIFF'): 'MAKE GM USER',
    ('start_event', 'btn_royal'): 'Open Ultima Register', ('start_event', 'btn_beef'): 'Open Beef Event',
    ('start_event', 'btn_jr'): 'Open Juraid Mountain', ('start_event', 'btn_bdw'): 'Open Border Defense War',
    ('start_event', 'btn_CR'): 'Open Collection Race',
    ('start_event', 'btn_adream'): 'Open FT', ('start_event', 'btn_closeadream'): 'Close FT',
    ('start_event', 'btn_cz'): 'Open Cindirella', ('start_event', 'btn_closecz'): 'Close Cind.',
    ('start_event', 'btn_base'): 'Open Manes', ('start_event', 'btn_closebase'): 'End Manes',
    ('start_event', 'btn_UTC'): 'Start Manes Survival', ('start_event', 'btn_DM'): 'Start Ultima Dungeon',
    ('start_event', 'btn_Clanws'): 'End', ('start_event', 'btn_CloseEvents'): 'Close All Event',
    ('gm_commands', 'btn_offsanta'): 'Off Santa / Angel', ('gm_commands', 'btn_oangel'): 'Open Angel',
    ('gm_commands', 'btn_discount'): 'Winner Discount', ('gm_commands', 'btn_down'): 'Off Discount',
    ('start_bots', 'btn_bot'): 'Send Bot Game',
}
STR_TEXT = {    # (page, original text) -> new text ('' hides)
    ('home', 'Your Role List'): 'Quick Actions',
    ('start_event', 'Minutes'): '', ('start_event', 'Clan VS Event'): 'Ultima Dungeon', ('start_event', 'War Zone:'): 'War Zone:',
    ('start_event', 'Target ID'): 'Event ID', ('start_event', 'Reward ID'): '',
    ('start_lottery', 'TICKET'): 'LOTTERY ID', ('start_lottery', 'TIME FOR MINUTE'): '', ('start_lottery', 'REQUIRED MONEY'): '',
    ('start_lottery', 'TOTAL WINNER'): '', ('start_lottery', 'CASH-NOAH'): '', ('start_lottery', 'REWARD ID'): '',
    ('start_lottery', 'Reward IDs 1 cash ,2 coins'): 'Settings come from the LOTTERY table',
    ('start_bots', '1.Type For Mining Bots'): '1. Mining   2. Fishing   3. Farm', ('start_bots', '2.Type For Fishing Bots'): '4. PK Karus   5. PK Human',
    ('start_bots', '4.Type For Stanting Bots'): '6. Fill the open event (Count)', ('start_bots', '5.Type For Stining Bots'): '7. Merchant bot (Count = index)',
    ('start_bots', '6.Type For Farmer Bots'): '8. Remove selected bot   9. Remove all', ('start_bots', 'Merchant Bots for auto starter GM'): 'Leader / Genie / Class:',
    ('start_bots', ' In Merchant item.'): 'HopeGuard tab, Bot Options', ('start_bots', 'In Merchant item.'): 'HopeGuard tab, Bot Options', ('start_bots', 'Bot Type'): 'Bot Type', ('start_bots', 'Min Level Bot'): 'Min Level',
}
# second-column minute captions of the special event rows: Type / ID / (none)
SPECIAL_EDIT_HINT = {'edit_minuteadream': 'type', 'edit_minutecz': 'id'}
# gm_commands notice type list (12 strings in reading order) -> our types
NOTICE_TYPES = ['Type 1 -> Notice (/notice)', 'Type 2 -> PM to every player', 'Type 3 -> Notice to all servers',
                'Type 4 -> Permanent top bar', 'Type 5 -> Remove the top bar', 'Message starting with + or / = GM command',
                '', '', '', '', '', '']
RELOADS = [('PUS', '+reloadpus'), ('ITEMS', '+reloaditems'), ('MAGICS', '+reloadmagics'), ('QUESTS', '+reloadquests'),
           ('UPGRADE', '+reloadupgrade'), ('DUNGEON', '+reloaddungeon'), ('DRAKI', '+reloaddraki'), ('EVENT', '+reloadevent'),
           ('TABLES 1', '+reloadtables'), ('TABLES 2', '+reloadtables2'), ('TABLES 3', '+reloadtables3'), ('DROP', '+reloaddrops'),
           ('R. DROP', '+reloaddrops2'), ('RANKS', '+reloadranks'), ('KINGS', '+reloadkings'), ('ITEM SELL', '+reloaditemsell'),
           ('CINDIRELLA', '+reload_cind'), ('LEVEL UP', '+reloadlevelup'), ('PREMIUM', '+reloadpremium'), ('NOTICE', '+reloadnotice'),
           ('TITLE', '+reloadtitle'), ('LOT. REW.', '+reloadlreward'), ('MANES REW.', '+reloadmreward'), ('ALL', '+reloadalltables')]
QUICK = ['Party Summon', 'Player Info', 'Disable Attack', 'Allow Attack', 'Jail', 'Unban', 'Exp Event Off',
         'Ride Horse (GM)', 'Online Count', 'GM Window', 'Remove Top Bar', 'Close Beef', 'Close Juraid', 'Close Border',
         'Close Chaos', 'Close FT', 'Close Lottery', 'All Bonus Off', 'Kill Target NPC', 'Remove Bot', 'Remove All Bots',
         'Close Cindirella', 'Close Coll. Race', 'Zone Population', 'Close War', 'Close CSW', 'End Ultima', 'End Manes']

# ------------------------------------------------------------------ walk
root = read_uif(UIF)
bg = {p: [] for p in ['frame'] + PAGES}          # static images per page
buttons, strings, edits, rows = [], [], [], []
seen_btn = set()
def walk(e, page):
    if e['id'] in PAGES: page = e['id']
    t = e['type']
    if t == 'BUTTON':
        key = (page, tuple(e['rect']))
        if key in seen_btn: return                   # reload page: btn_royal / btn_drop on the same spot
        seen_btn.add(key)
        st = {k['res']: k for k in e['children'] if k['type'] == 'IMAGE' and k.get('tex')}
        lab = [k for k in e['children'] if k['type'] == 'STRING']
        buttons.append({'page': page, 'id': e['id'] or '-', 'rect': e['rect'], 'states': st, 'label': lab[0] if lab else None})
        return
    if t == 'EDIT':
        for k in e['children']:
            if k['type'] == 'IMAGE' and k.get('tex'): bg[page].append(k)
        hint = [k['text'] for k in e['children'] if k['type'] == 'STRING']
        edits.append({'page': page, 'id': e['id'], 'rect': e['rect'], 'hint': hint[0] if hint else ''})
        return
    if t == 'IMAGE' and e.get('tex'): bg[page].append(e)
    if t == 'STRING': strings.append({'page': page, 'id': e['id'], 'rect': e['rect'], 'text': e['text'], 'color': e['color'],
                                      'fontH': e['fontH'], 'style': e['style']})
    for k in e['children']: walk(k, page)
for e in root: walk(e, 'frame')

# home role list -> quick-action rows (row plates of role_list, column-major like the original names)
plates = [i for i in bg['home'] if i['tex'].lower().endswith('re_loyalty_mark.dxt') and i['rect'][3] - i['rect'][1] == 16]
plates.sort(key=lambda i: (i['rect'][0], i['rect'][1]))
for n, p in enumerate(plates[:len(QUICK)]): rows.append({'rect': p['rect'], 'label': QUICK[n]})
strings = [s for s in strings if not (s['page'] == 'home' and (s['id'].startswith(('txt_s', 'on_', 'off_')) or s['id'] in ('txt_GiveItemSelf', 'txt_GiveItem')))]
# role-name strings without the txt_s prefix
role_names = {'Un Mute', 'Un Ban', 'Ban Permit', 'Allow Attack', 'Disabled Attack', 'Np Add', 'Exp Add', 'Money Add', 'Exp Change',
              'Money Change', 'Give Item', 'Give Item Self', 'Summon User', 'Tp On User', 'Zone Change', 'Location Change',
              'Monster Summon', 'NPC Summon', 'Mon Killed', 'Teleport All User', 'Clan Summon', 'Reset Ranking', 'War Opened',
              'War Closed', 'Cash Added', 'Ban Cheating', 'Active'}
def in_role_list(s):
    l, t, r, b = s['rect']; return 437 <= l and r <= 815 and 236 <= t and b <= 500
strings = [s for s in strings if not (s['page'] == 'home' and in_role_list(s) and (s['text'] in role_names or not s['text'].strip() or True))]

# relabel strings
for s in strings:
    k = (s['page'], s['text'])
    if k in STR_TEXT: s['text'] = STR_TEXT[k]
# user editor: the weapon AC rows hold the skill points (editable), HP / MP rows STA / CHA; label boxes widened to the value
UE_LABELS = {'Dagger:': ('Free SP:', 163), 'Sword:': ('Skill 1:', 163), 'Mace:': ('Skill 2:', 163), 'Spear:': ('Skill 3:', 290),
             'Axe:': ('Master:', 290), 'Bow:': ('HP:', 290), 'HP:': ('STA:', 292), 'MP:': ('CHA:', 292)}
for s in strings:
    if s['page'] == 'user_editor' and not s['id'] and s['text'] in UE_LABELS:
        s['text'], right = UE_LABELS[s['text']]; s['rect'] = [s['rect'][0], s['rect'][1], right, s['rect'][3]]
nt = [s for s in strings if s['page'] == 'gm_commands' and 'Type ->' in s['text']]
nt.sort(key=lambda s: (s['rect'][1] // 20, s['rect'][0]))
for s, txt in zip(nt, NOTICE_TYPES): s['text'] = txt; s['color'] = 0xFFFFFFFF if txt else s['color']
# reload books: name string above each RELOAD button, in reading order
rb = sorted([b for b in buttons if b['page'] == 'reload_tables'], key=lambda b: (b['rect'][1] // 50, b['rect'][0]))
names = [s for s in strings if s['page'] == 'reload_tables' and s['text'] not in ('TABLE', 'RELOAD') and s['rect'][1] > 190]
reload_cmds = []
for n, b in enumerate(rb):
    bx = (b['rect'][0] + b['rect'][2]) / 2
    cand = [s for s in names if s['rect'][3] <= b['rect'][1] and b['rect'][1] - s['rect'][3] < 60 and abs((s['rect'][0] + s['rect'][2]) / 2 - bx) < 45]
    cand.sort(key=lambda s: b['rect'][1] - s['rect'][3])
    lab, cmd = RELOADS[n] if n < len(RELOADS) else ('', '')
    if cand: cand[0]['text'] = lab
    b['id'] = 'reload%d' % n; reload_cmds.append(cmd)
for b in buttons:
    if b['label'] is not None and (b['page'], b['id']) in BTN_LABEL: b['label']['text'] = BTN_LABEL[(b['page'], b['id'])]
for e in edits:
    if e['id'] in SPECIAL_EDIT_HINT: e['hint'] = SPECIAL_EDIT_HINT[e['id']]
DISABLED_EDITS = {'edit_minutebase', 'edit_warzoneclan', 'edit_clan1', 'edit_clan2', 'edit_Minutecr', 'edit_odulcr',
                  'edit_time', 'edit_money', 'edit_totalwinner', 'edit_kc', 'edit_rewardid'}
HINTS = {'edit_warzone': '1-6', 'edit_hedefcr': 'id', 'Edit_ticket': 'id', 'edit_banday': 'days', 'edit_type': '1', 'edit_mesaj': 'Message',
         'edit_count': '1', 'edit_Time': '60', 'edit_Type': '1-9', 'edit_lvl': '1', 'edit_search': 'name', 'edit_warresult': '1/2',
         'edit_zone1': 'zone', 'edit_zone2': 'zone'}
edits = [e for e in edits if e['id'] not in DISABLED_EDITS]
for e in edits: e['hint'] = HINTS.get(e['id'], e['hint'])
# unnamed scroll buttons of the user list
for b in buttons:
    if b['page'] == 'user_editor' and b['id'] == '-': b['id'] = 'scroll_up' if b['rect'][1] < 250 else 'scroll_down'
buttons = [b for b in buttons if not (b['page'] == 'gm_commands' and b['id'] == 'btn_offangel')]   # same spot as Open Angel

# ------------------------------------------------------------------ 8th tab "HopeGuard": our tools the cyber window has no
# place for, laid out with the same cyber pieces as the Send Bot page (page plate, list box, title plate, row plate, edit box,
# Send Bot button)
def img(tex, uv, rect): return {'type': 'IMAGE', 'tex': 'ui\\' + tex, 'uv': uv, 'rect': list(rect), 'children': []}
BASE = [i for i in bg['start_bots'] if i['tex'].lower().endswith('re_akara_01.dxt')][0]
BOX = ('re_skill04.dxt', [0.0957, 0.8945, 0.2686, 0.9668])
TITLE = ('re_skill04.dxt', [0.0605, 0.8242, 0.2002, 0.8525])
HEAD = ('re_skill04.dxt', [0.0371, 0.8242, 0.2373, 0.8525])
PLATE = ('re_skill04.dxt', [0.0557, 0.8242, 0.1875, 0.8525])
EDITBG = ('re_auction_01.dxt', [0.168, 0.0645, 0.459, 0.1152])
BOTBTN = [b for b in buttons if b['page'] == 'start_bots' and b['id'] == 'btn_bot'][0]
PAGES.append('extra'); bg['extra'] = [dict(BASE)]
def xstr(rect, text, h=10, col=0xFFFFFFFF, style=0x4900000):
    strings.append({'page': 'extra', 'id': '', 'rect': list(rect), 'text': text, 'color': col, 'fontH': h, 'style': style})
def xhead(rect, text): bg['extra'].append(img(*HEAD, rect)); xstr((rect[0] + 8, rect[1] + 5, rect[2] - 8, rect[3] - 5), text, 11)
def xbox(rect, title):
    bg['extra'].append(img(*BOX, rect)); cx = (rect[0] + rect[2]) // 2
    bg['extra'].append(img(*TITLE, (cx - 90, rect[1] + 6, cx + 90, rect[1] + 35))); xstr((cx - 86, rect[1] + 12, cx + 86, rect[1] + 29), title)
def xplate(x, y, text, w=116): bg['extra'].append(img(*PLATE, (x, y + 2, x + w, y + 24))); xstr((x + 2, y + 5, x + w - 2, y + 21), text)
def xedit(id_, x0, x1, y, hint=''):
    bg['extra'].append(img(*EDITBG, (x0, y, x1, y + 26))); edits.append({'page': 'extra', 'id': id_, 'rect': [x0, y, x1, y + 26], 'hint': hint})
def xbtn(id_, x0, x1, y, label):
    rect = [x0, y - 1, x1, y + 27]
    st = {k: dict(v, rect=rect) for k, v in BOTBTN['states'].items()}
    lab = {'text': label, 'rect': [x0 + 4, y + 4, x1 - 4, y + 22], 'color': 0xFFFFFFFF, 'fontH': 10, 'style': 0x4900000}
    buttons.append({'page': 'extra', 'id': id_, 'rect': rect, 'states': st, 'label': lab})
xhead((140, 141, 780, 170), 'HopeGuard tools  -  player commands use the player selected in User Editor')
L0, L1, R0, R1 = 68, 462, 470, 862
xbox((L0, 180, L1, 350), 'Selected Player')
y = 225; xplate(80, y, 'Amount'); xedit('x_amount', 200, 300, y, 'amount'); xbtn('x_info', 305, 455, y, 'Refresh Info')
y = 263; xbtn('x_level', 80, 170, y, 'Level'); xbtn('x_np', 175, 265, y, 'NP'); xbtn('x_kc', 270, 360, y, 'KC'); xbtn('x_exp', 365, 455, y, 'EXP')
y = 301; xplate(80, y, 'Item ID'); xedit('x_itemid', 200, 272, y, 'item id'); xedit('x_itemcount', 276, 316, y, 'count'); xedit('x_itemtime', 320, 360, y, 'days'); xbtn('x_giveitem', 364, 455, y, 'Give Item')
xbox((L0, 360, L1, 548), 'Game Master')
y = 403; xplate(80, y, 'Zone'); xedit('x_zone', 200, 290, y, 'zone'); xbtn('x_gozone', 295, 455, y, 'Go To Zone')
y = 438; xplate(80, y, 'Item ID'); xedit('x_myitem', 200, 290, y, 'item id'); xedit('x_myitemcount', 295, 345, y, 'count'); xbtn('x_giveself', 350, 455, y, 'Give Self')
y = 473; xplate(80, y, 'Bowl Event'); xedit('x_bowlzone', 200, 260, y, 'zone'); xedit('x_bowltime', 265, 325, y, 'min'); xbtn('x_bowl', 330, 455, y, 'Start Bowl')
y = 508; xplate(80, y, 'Command'); xedit('x_cmd', 200, 370, y, '+command / /command'); xbtn('x_run', 375, 455, y, 'Run')
xbox((R0, 180, R1, 350), 'Bonus Events')
y = 225; xplate(482, y, 'Percent'); xedit('x_bonus', 602, 692, y, '%'); xbtn('x_bonusoff', 697, 852, y, 'All Bonus Off')
y = 263; xbtn('x_bexp', 482, 572, y, 'EXP Event'); xbtn('x_bnp', 575, 665, y, 'NP Event'); xbtn('x_bcoin', 668, 758, y, 'Coin Event'); xbtn('x_bdrop', 761, 852, y, 'Drop Event')
y = 301; xplate(482, y, 'Zone'); xedit('x_popzone', 602, 692, y, 'zone'); xbtn('x_pop', 697, 852, y, 'Zone Population')
xbox((R0, 360, R1, 548), 'PM / Tables / Bot Options')
y = 403; xplate(482, y, 'PM Title'); xedit('x_pmtitle', 602, 852, y, 'title')
y = 438; xplate(482, y, 'PM Message'); xedit('x_pmmsg', 602, 772, y, 'message'); xbtn('x_pmsend', 777, 852, y, 'Send PM')
y = 473
for n, (id_, lab) in enumerate([('x_r_social', 'Social'), ('x_r_bug', 'Upg. Bug'), ('x_r_clanp', 'Clan Notice'), ('x_r_item', 'Reload Item'), ('x_r_zoneon', 'Zone On')]):
    xbtn(id_, 482 + n * 74, 552 + n * 74, y, lab)
y = 508
for n, (id_, hint) in enumerate([('x_bleader', 'farm leader 0/1'), ('x_bgenie', 'farm genie 0/1'), ('x_bclass', 'bot class (0 = any)')]):
    xedit(id_, 482 + n * 124, 600 + n * 124, y, hint)
# tab bar: 8 tabs (cyber order + HopeGuard) over the same strip
tabs = ['btn_home', 'btn_table', 'btn_events', 'btn_commands', 'btn_lottery', 'btn_bot', 'btn_user_editor']
tb = {b['id']: b for b in buttons if b['page'] == 'frame' and b['id'] in tabs}
src = tb['btn_user_editor']
nb = {'page': 'frame', 'id': 'btn_extra', 'rect': list(src['rect']), 'states': {k: dict(v) for k, v in src['states'].items()},
      'label': dict(src['label'], text='HopeGuard')}
buttons.append(nb); tb['btn_extra'] = nb
TW = 101
for n, id_ in enumerate(tabs + ['btn_extra']):
    b = tb[id_]; l = 62 + n * TW; r = [l, 82, l + TW, 129]
    b['rect'] = r
    for k in b['states']: b['states'][k] = dict(b['states'][k], rect=r)
    b['label'] = dict(b['label'], rect=[l + 6, 100, l + TW - 6, 119])
PIDX = {'frame': -1, **{p: i for i, p in enumerate(PAGES)}}

# ------------------------------------------------------------------ sprites
allimgs =[i for p in bg.values() for i in p] + [s for b in buttons for s in b['states'].values()]
W = max(i['rect'][2] for i in allimgs) + 4; H = max(i['rect'][3] for i in allimgs) + 4
sprites = []
def compose(name, imgs):
    if not imgs: return None
    l = min(i['rect'][0] for i in imgs); t = min(i['rect'][1] for i in imgs)
    r = max(i['rect'][2] for i in imgs); b = max(i['rect'][3] for i in imgs)
    cv = Image.new('RGBA', (r - l, b - t), (0, 0, 0, 0))
    for i in imgs:
        if i['rect'][2] > i['rect'][0] and i['rect'][3] > i['rect'][1]:
            cv.alpha_composite(piece(i), (i['rect'][0] - l, i['rect'][1] - t))
    save_pus(cv, name); sprites.append((name, cv.size[0], cv.size[1]))
    return (name, l, t)
bgs = {p: compose('bg_' + p, imgs) for p, imgs in bg.items()}
bspr = {}
def btn_sprite(e):
    key = (os.path.basename(e['tex']).lower(), tuple(round(x, 4) for x in e['uv']), e['rect'][2] - e['rect'][0], e['rect'][3] - e['rect'][1])
    if key not in bspr:
        name = 'b%03d' % len(bspr); img = piece(e); save_pus(img, name); sprites.append((name, img.size[0], img.size[1])); bspr[key] = name
    return bspr[key]
for b in buttons:
    b['spr'] = [btn_sprite(b['states'][s]) if s in b['states'] else None for s in range(4)]
    if not b['spr'][2]: b['spr'][2] = b['spr'][0]
    if not b['spr'][1]: b['spr'][1] = b['spr'][2]

# ------------------------------------------------------------------ header
def cstr(s): return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'
def wstr(s): return 'L' + cstr(s.encode('ascii', 'replace').decode())
PIDX = {'frame': -1, **{p: i for i, p in enumerate(PAGES)}}
L = ['// Generated by build_gm_assets.py from cyber001 CyberACS\\Dat\\DAT-19.uif -- do not edit by hand.', '#pragma once',
     'static const int GMU_W = %d, GMU_H = %d;' % (W, H),
     'struct GmuSprite { const char* name; int w, h; };', 'static const GmuSprite GMU_SPRITES[] = {']
L += ['    { %s, %d, %d },' % (cstr(n), w, h) for n, w, h in sprites] + ['};']
L += ['struct GmuBg { int page; const char* spr; int x, y; };', 'static const GmuBg GMU_BG[] = {']
L += ['    { %d, %s, %d, %d },' % (PIDX[p], cstr(v[0]), v[1], v[2]) for p, v in bgs.items() if v] + ['};']
L += ['struct GmuBtn { int page; const char* id; int l, t, r, b; const char* spr[4]; const wchar_t* label; int ll, lt, lr, lb; unsigned color; int fontH; unsigned style; };',
      'static const GmuBtn GMU_BTNS[] = {']
for b in buttons:
    lab = b['label'] or {'text': '', 'rect': b['rect'], 'color': 0xFFFFFFFF, 'fontH': 10, 'style': 0x4900000}
    L.append('    { %d, %s, %d, %d, %d, %d, { %s }, %s, %d, %d, %d, %d, 0x%08X, %d, 0x%08X },' % (
        PIDX[b['page']], cstr(b['id']), *b['rect'], ', '.join(cstr(s) if s else 'nullptr' for s in b['spr']), wstr(lab['text']), *lab['rect'],
        lab['color'], lab['fontH'], lab['style']))
L += ['};', 'struct GmuStr { int page; const char* id; int l, t, r, b; const wchar_t* text; unsigned color; int fontH; unsigned style; };',
      'static const GmuStr GMU_STRS[] = {']
for s in strings:
    if not s['text'].strip() and not s['id']: continue
    L.append('    { %d, %s, %d, %d, %d, %d, %s, 0x%08X, %d, 0x%08X },' % (PIDX[s['page']], cstr(s['id']), *s['rect'], wstr(s['text']), s['color'], s['fontH'], s['style']))
L += ['};', 'struct GmuEdit { int page; const char* id; int l, t, r, b; const wchar_t* hint; };', 'static const GmuEdit GMU_EDITS[] = {']
L += ['    { %d, %s, %d, %d, %d, %d, %s },' % (PIDX[e['page']], cstr(e['id']), *e['rect'], wstr(e['hint'])) for e in edits] + ['};']
L += ['struct GmuRow { int l, t, r, b; const wchar_t* label; };', 'static const GmuRow GMU_ROWS[] = {']
L += ['    { %d, %d, %d, %d, %s },' % (*r['rect'], wstr(r['label'])) for r in rows] + ['};']
L += ['static const char* const GMU_RELOAD_CMDS[] = { %s };' % ', '.join(cstr(c) for c in reload_cmds)]
open(os.path.join(HERE, 'gm_layout.h'), 'w', newline='\r\n').write('\n'.join(L) + '\n')
print('window %dx%d, %d sprites, %d buttons, %d strings, %d edits, %d quick rows, %d reloads' % (
    W, H, len(sprites), len(buttons), len(strings), len(edits), len(rows), len(reload_cmds)))
