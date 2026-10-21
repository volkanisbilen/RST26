@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars32.bat" >nul
if not exist ..\..\_obj mkdir ..\..\_obj
cl /nologo /EHsc /Fo..\..\_obj\ overlay_math_test.cpp /Fe..\..\_obj\ovt.exe >nul
if errorlevel 1 exit /b 1
..\..\_obj\ovt.exe
