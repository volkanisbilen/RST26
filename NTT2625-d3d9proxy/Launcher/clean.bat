@echo off
rmdir /s /q ".vs" 2>nul
rmdir /s /q "Debug" 2>nul
rmdir /s /q "Release" 2>nul
rmdir /s /q "x64" 2>nul
rmdir /s /q "Win32" 2>nul
rmdir /s /q "WIN32" 2>nul
rmdir /s /q "Debug_x86" 2>nul
rmdir /s /q "Debug_x64" 2>nul
rmdir /s /q "Release_x86" 2>nul
rmdir /s /q "Release_x64" 2>nul
rmdir /s /q "Intermediate" 2>nul
rmdir /s /q "ipch" 2>nul
del /s /q "*.obj" "*.pdb" "*.ilk" "*.log" "*.tlog" "*.lastbuildstate" 2>nul
echo Clean completed.
