@echo off
rem Builds Launcher.exe (splash + patch pull + cheat scan + start game).
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars32.bat" >nul
cl /nologo /std:c++17 /O2 /MT /EHsc launcher.cpp launcher_patch.cpp /Fe:Launcher.exe /link /SUBSYSTEM:WINDOWS gdiplus.lib user32.lib gdi32.lib shell32.lib psapi.lib winhttp.lib bcrypt.lib
