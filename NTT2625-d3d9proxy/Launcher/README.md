# JstKO Launcher Source

Bu paket NLauncher gorselleri exe icine gomulu ve x86/x64 Release build alabilecek sekilde revize edilmis kaynak pakettir.

Onemli notlar:
- NLauncher PNG dosyalari Launcher.rc icinde RCDATA olarak exe kaynaklarina gomulur.
- Launcher.exe calisirken SRGAME\\NLauncher veya disaridan gorsel klasoru istemez.
- NLauncher klasoru kaynak build icin projede kalmalidir.
- x86 ve x64 Release cikislari ayri klasorlere gider: Release_x86, Release_x64.
- libcurl ve x86-only d3dx9 import lib bagimliligi kaldirildi.

Detayli degisiklikler icin:
- JSTKO_NLAUNCHER_EMBED_NOTLAR.txt
- JSTKO_NLAUNCHER_X86_X64_RELEASE_NOTLAR.txt
