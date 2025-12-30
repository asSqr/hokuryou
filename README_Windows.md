## MySQL 立ち上げ方法
以下を 管理者 PowerShell で実行する：
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force
.\scripts\setup-docker-and-compose.ps1 -ComposeDir "C:\path\to\your\project"
```