@echo off
rem Builds HopeGuard.dll (x86, formerly OPSGUARD.dll) and the offline tests (test\). Does not copy anything to the client.
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars32.bat" >nul
if errorlevel 1 exit /b 1
cd /d "%~dp0"
cl /nologo /std:c++17 /LD /O2 /MT /GS- opsguard.cpp /link /OUT:HopeGuard.dll /IMPLIB:HopeGuard.lib
if errorlevel 1 exit /b 1
if not exist test mkdir test
cl /nologo /std:c++17 /O2 /MT /EHsc /Fotest\ test_host.cpp /link /INCREMENTAL:NO /OUT:test\test_host.exe
if errorlevel 1 exit /b 1
cl /nologo /std:c++17 /O2 /MT /EHsc /Fotest\ test_host2.cpp /link /INCREMENTAL:NO /OUT:test\test_host2.exe
if errorlevel 1 exit /b 1
cl /nologo /std:c++17 /LD /O2 /MT /Fotest\ test_proxy.cpp HopeGuard.lib /link /OUT:test\d3d9.dll
if errorlevel 1 exit /b 1
copy /y HopeGuard.dll test\ >nul
