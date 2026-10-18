DAILY QUEST MANAGER CLIENT PATCH

Copy these files into the active client Data directory, replacing the existing
files after making a backup:

- NPC_us.tbl (required for the active English client)
- NPC_tk.tbl (Turkish client fallback)
- NPC.tbl (base fallback)

The patch adds only client proto 31999:
  [Daily Quest Manager] Aelion, visual PID 30500.

Proto 29514 [Scroll Merchant] is already present in the supplied v2615
NPC_us.tbl; it is intentionally not duplicated by this patch.
