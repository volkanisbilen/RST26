@echo off
cd /d "%~dp0"
python hgpus.py encode
python hgpus.py install
pause
