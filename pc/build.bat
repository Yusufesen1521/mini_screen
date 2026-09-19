@echo off
rem VS 18 kurulumunda msvcrt.lib eksik, VS 2022 ortamini zorluyoruz.
rem Bu betik gecici: makinedeki VS 18 kurulumu duzelirse gerek kalmaz.
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0"
cargo %*
