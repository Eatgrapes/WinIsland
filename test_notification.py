import sys
import subprocess

def send_toast_powershell(title, body):
    """使用 PowerShell 发送 toast 通知 - 长时间显示"""
    ps_script = f'''
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null

$template = [Windows.UI.Notifications.ToastTemplateType]::ToastText02
$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent($template)
$textElements = $xml.GetElementsByTagName("text")
$textElements.Item(0).AppendChild($xml.CreateTextNode("{title}")) | Out-Null
$textElements.Item(1).AppendChild($xml.CreateTextNode("{body}")) | Out-Null

$notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App")
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
# 设置长显示时间
$toast.ExpirationTime = [DateTimeOffset]::Now.AddMinutes(30)
$toast.SuppressPopup = $false
$notifier.Show($toast)

# 保持脚本运行，让通知不消失
Write-Host "Notification sent. Press Enter to close..."
Read-Host
'''
    result = subprocess.run(['powershell', '-Command', ps_script], capture_output=True, text=True)
    return result.returncode == 0

if __name__ == "__main__":
    title = sys.argv[1] if len(sys.argv) > 1 else "Test Notification"
    body = sys.argv[2] if len(sys.argv) > 2 else "This is a test notification body"
    
    if send_toast_powershell(title, body):
        print(f"Sent notification: {title} - {body}")
    else:
        print("Failed to send notification")
        sys.exit(1)
