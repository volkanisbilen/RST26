@echo off
rem Builds the d3d9 proxy (x86) and deploys it to the unpack client folder.
setlocal
set GAMEDIR=F:\KnightOnlineEn - Kopya\KnightOnlineEn - Kopya

call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars32.bat" >nul
if errorlevel 1 goto :fail

cd /d "%~dp0"
cl /nologo /std:c++17 /LD /O2 /EHsc /MT d3d9proxy.cpp pus_store.cpp rce_store.cpp cr_panel.cpp cape_gate.cpp crash_trap.cpp iat_fix.cpp client_patches.cpp sp_panel.cpp genie_hg.cpp cind_panel.cpp lottery_panel.cpp event_reward_panel.cpp anvil_rate.cpp login_remember.cpp drop_panel.cpp cross_exch.cpp reconnect.cpp hold_repeat.cpp net_trace.cpp manner_store.cpp hotkey_guard.cpp dropbox_panel.cpp launch_guard.cpp tag_panel.cpp hopeguard_banner.cpp /link /DEF:d3d9.def user32.lib gdi32.lib msimg32.lib crypt32.lib /OUT:d3d9.dll
if errorlevel 1 goto :fail

if exist "%GAMEDIR%\d3d9.dll" copy /y "%GAMEDIR%\d3d9.dll" "%GAMEDIR%\d3d9.dll.bak" >nul
copy /y d3d9.dll "%GAMEDIR%\d3d9.dll"
if errorlevel 1 (
    echo.
    echo KOPYALAMA BASARISIZ - oyun acik olabilir, kapatip tekrar deneyin.
    goto :end
)
echo.
echo TAMAM: d3d9.dll derlendi ve "%GAMEDIR%" klasorune kopyalandi.
goto :end

:fail
echo.
echo DERLEME BASARISIZ - yukaridaki hatayi Claude'a yapistirin.

:end
pause
