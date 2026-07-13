//! y5 i18n core — compile-time-resolved locale lookup.
//!
//! ## Locale detection
//!
//! `is_zh()` reads the `LANG` environment variable **once** (cached via
//! `OnceLock`) on first call and returns `true` when the value starts with
//! `zh` (case-insensitive).  All subsequent calls are a single atomic load.
//!
//! ## Translation macro
//!
//! `t!("key")` expands to a `&'static str`.  When `is_zh()` returns `true`
//! the macro calls `zh(key)` which match-es against the translation table
//! and returns the Chinese string or the original key as fallback.  When
//! `is_zh()` returns `false` the macro returns the key verbatim **with zero
//! overhead** (no table lookup).
//!
//! ## Variable interpolation
//!
//! The return value is a plain `&str`.  Callers who need formatting compose
//! it with `format!()`:
//!
//! ```
//! text(format!(t!("fps_template"), fps = fps))
//! ```
//!
//! This keeps the translation keys simple and avoids pulling formatting
//! logic into the match table.

use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locale detection
// ---------------------------------------------------------------------------

static LOCALE_IS_ZH: OnceLock<bool> = OnceLock::new();

/// Returns `true` when the `LANG` environment variable starts with `zh`.
///
/// The value is computed once and cached for the lifetime of the process.
/// Use this directly only when you need a boolean branch; for string
/// translation always prefer the [`t!`] macro.
#[inline]
pub fn is_zh() -> bool {
    *LOCALE_IS_ZH.get_or_init(|| {
        std::env::var("LANG")
            .map(|v| v.starts_with("zh"))
            .unwrap_or(false)
    })
}

/// Convenience wrapper — returns `t!("key")` at runtime.
///
/// Useful in contexts where a function call is more ergonomic than a macro
/// invocation (e.g., builder patterns, method chaining).
#[inline]
pub fn tr(key: &'static str) -> &'static str {
    t!(key)
}

// ---------------------------------------------------------------------------
// Translation macro
// ---------------------------------------------------------------------------

/// Resolve a translatable string key to a `&'static str`.
///
/// In the zh locale the key is looked up in the Chinese translation table;
/// otherwise it is returned unchanged (zero-cost identity).
#[macro_export]
macro_rules! t {
    ($key:expr) => {{
        if $crate::is_zh() {
            $crate::zh($key)
        } else {
            $key
        }
    }};
}

// ---------------------------------------------------------------------------
// Chinese translation table
// ---------------------------------------------------------------------------

/// Look up `key` in the Chinese translation table.
///
/// Returns the Chinese string when found, or the original `key` as a
/// fallback so missing translations degrade to English rather than blank
/// or panicked output.
#[inline]
pub fn zh(key: &str) -> &str {
    match key {
        // -- General UI ----------------------------------------------------
        "Cancel" => "取消",
        "Close" => "关闭",
        "Confirm" => "确认",
        "Continue" => "继续",
        "Default" => "默认",
        "Delete" => "删除",
        "Details" => "详情",
        "Disabled" => "已禁用",
        "Discard" => "放弃",
        "Dismiss" => "关闭",
        "Enabled" => "已启用",
        "Error" => "错误",
        "ESC" => "ESC",
        "Exit" => "退出",
        "General" => "通用",
        "Info" => "信息",
        "Manual" => "手动",
        "Name" => "名称",
        "None" => "无",
        "Off" => "关",
        "On" => "开",
        "Overview" => "概览",
        "Pause" => "暂停",
        "Reset" => "重置",
        "Restore defaults" => "恢复默认设置",
        "Retry" => "重试",
        "Save" => "保存",
        "Search" => "搜索",
        "Show" => "显示",
        "Skip" => "跳过",
        "Submit" => "提交",
        "Tip" => "提示",
        "Unknown" => "未知",
        "Unlimited" => "无限制",
        "Untitled" => "未命名",
        "Warning" => "警告",

        // -- Settings sidebar / categories ---------------------------------
        "Action" => "动作",
        "Actions" => "动作",
        "Appearance" => "外观",
        "Applications" => "应用程序",
        "Configuration" => "配置",
        "Desktop" => "桌面",
        "Extensions" => "扩展",
        "Input" => "输入",
        "Interface" => "界面",
        "Keyboard" => "键盘",
        "Notifications" => "通知",
        "Output" => "输出",
        "Plugins" => "插件",
        "Power" => "电源",
        "Privacy" => "隐私",
        "Region & Language" => "区域和语言",
        "Screen" => "屏幕",
        "Security" => "安全",
        "Settings" => "设置",
        "Sound" => "声音",
        "System" => "系统",
        "Updates" => "更新",
        "Workspace" => "工作区",
        "Workspaces" => "工作区",

        // -- Overview / search ---------------------------------------------
        "Overview Mode" => "概览模式",
        "Search applications and files…" => "搜索应用程序和文件…",
        "Type a command…" => "输入命令…",
        "Switch Window" => "切换窗口",
        "Switch Workspace" => "切换工作区",
        "Launch Application" => "启动应用程序",
        "Run Command" => "运行命令",
        "No results found" => "未找到结果",
        "Window" => "窗口",
        "Windows" => "窗口",

        // -- Keyboard shortcuts tab ----------------------------------------
        "Keybindings" => "快捷键",
        "Shortcut" => "快捷键",
        "Shortcuts" => "快捷键",
        "Combo" => "组合键",
        "Action" => "动作",
        "Open Settings" => "打开设置",
        "Toggle Overview" => "切换概览",
        "Focus Workspace 1–9" => "聚焦工作区 1–9",
        "Move Window to Workspace 1–9" => "移动窗口到工作区 1–9",
        "Switch to Next Workspace" => "切换到下一个工作区",
        "Switch to Previous Workspace" => "切换到上一个工作区",
        "Close Window" => "关闭窗口",
        "Fullscreen" => "全屏",
        "Maximize" => "最大化",
        "Minimize" => "最小化",
        "Tile Left" => "平铺到左侧",
        "Tile Right" => "平铺到右侧",
        "Toggle Floating" => "切换浮动模式",
        "Launch Terminal" => "启动终端",
        "Launch Application Launcher" => "启动应用启动器",
        "Screenshot" => "截图",
        "Screenshot Area" => "区域截图",
        "Lock Screen" => "锁定屏幕",
        "Log Out" => "注销",
        "Suspend" => "暂停",
        "Hibernate" => "休眠",
        "Power Off" => "关机",
        "Reboot" => "重启",
        "Show Help" => "显示帮助",

        // -- Display / output tab ------------------------------------------
        "Display" => "显示",
        "Resolution" => "分辨率",
        "Refresh Rate" => "刷新率",
        "Orientation" => "方向",
        "Landscape" => "横向",
        "Portrait" => "纵向",
        "Landscape (flipped)" => "横向（翻转）",
        "Portrait (flipped)" => "纵向（翻转）",
        "Scale" => "缩放",
        "Scaling" => "缩放",
        "Auto" => "自动",
        "Fractional" => "分数缩放",
        "Mirror" => "镜像",
        "Extend" => "扩展",
        "Primary Display" => "主显示器",
        "Arrange displays" => "排列显示器",
        "Detect Displays" => "检测显示器",
        "Night Light" => "夜间模式",
        "Schedule" => "定时",
        "Sunset to Sunrise" => "日落到日出",
        "Custom Schedule" => "自定义时间",
        "Temperature" => "色温",
        "Brightness" => "亮度",

        // -- Input tab -----------------------------------------------------
        "Mouse" => "鼠标",
        "Touchpad" => "触摸板",
        "Touchscreen" => "触摸屏",
        "Pointer Speed" => "指针速度",
        "Natural Scrolling" => "自然滚动",
        "Tap to Click" => "轻触点击",
        "Two-Finger Scroll" => "双指滚动",
        "Edge Scrolling" => "边缘滚动",
        "Disable While Typing" => "输入时禁用",
        "Acceleration" => "加速度",
        "Primary Button" => "主按钮",
        "Left" => "左",
        "Right" => "右",
        "Middle-click Paste" => "中键粘贴",

        // -- Appearance tab ------------------------------------------------
        "Theme" => "主题",
        "Light" => "浅色",
        "Dark" => "深色",
        "Accent Color" => "强调色",
        "Font" => "字体",
        "Font Size" => "字号",
        "Interface Font" => "界面字体",
        "Monospace Font" => "等宽字体",
        "Cursor Size" => "光标大小",
        "Background" => "壁纸",
        "Wallpaper" => "壁纸",
        "Solid Color" => "纯色",
        "Lock Screen Wallpaper" => "锁屏壁纸",
        "Blur" => "模糊",
        "Opacity" => "不透明度",
        "Animations" => "动画",
        "Reduced Motion" => "减少动画",
        "Corner Radius" => "圆角半径",
        "Panel Position" => "面板位置",
        "Top" => "顶部",
        "Bottom" => "底部",
        "Panel Style" => "面板样式",

        // -- Extensions tab ------------------------------------------------
        "Browse Extensions" => "浏览扩展",
        "Install from File…" => "从文件安装…",
        "Extension" => "扩展",
        "Version" => "版本",
        "Author" => "作者",
        "Enabled" => "已启用",
        "Uninstall" => "卸载",
        "Configure" => "配置",
        "No extensions installed" => "未安装任何扩展",
        "Updates Available" => "有可用更新",

        // -- Notification settings tab -------------------------------------
        "Do Not Disturb" => "勿扰模式",
        "Show Notification Count" => "显示通知数",
        "Notification" => "通知",
        "Notifications" => "通知",
        "App Notifications" => "应用通知",
        "Notification Popups" => "通知弹窗",
        "Lock Screen Notifications" => "锁屏通知",
        "Show Content on Lock Screen" => "锁屏显示内容",
        "Hide Sensitive Content" => "隐藏敏感内容",
        "Critical Notifications Only" => "仅重要通知",

        // -- Privacy & Security tab ----------------------------------------
        "Location" => "位置",
        "Location Services" => "定位服务",
        "Camera" => "摄像头",
        "Microphone" => "麦克风",
        "Screen Recording" => "屏幕录制",
        "Remote Desktop" => "远程桌面",
        "File Sharing" => "文件共享",
        "Firewall" => "防火墙",
        "Screen Lock" => "屏幕锁定",
        "Lock Screen After" => "自动锁屏",
        "Never" => "从不",
        "1 Minute" => "1 分钟",
        "2 Minutes" => "2 分钟",
        "5 Minutes" => "5 分钟",
        "10 Minutes" => "10 分钟",
        "15 Minutes" => "15 分钟",
        "30 Minutes" => "30 分钟",
        "1 Hour" => "1 小时",
        "Require Password After Wake" => "唤醒后需要密码",
        "Immediately" => "立即",
        "Allow Screen Blanking" => "允许屏幕熄屏",
        "Authentication" => "认证",
        "Fingerprint" => "指纹",
        "Face Unlock" => "面部解锁",
        "PIN" => "PIN 码",
        "Password" => "密码",
        "Change Password" => "修改密码",
        "User Accounts" => "用户帐户",

        // -- Power tab -----------------------------------------------------
        "Power Mode" => "电源模式",
        "Balanced" => "均衡",
        "Performance" => "性能",
        "Power Saver" => "节能",
        "Battery" => "电池",
        "Battery Percentage" => "电池百分比",
        "Low Battery Warning" => "低电量提醒",
        "When Lid Is Closed" => "合盖时",
        "Sleep" => "睡眠",
        "Do Nothing" => "无操作",
        "Show Battery in Panel" => "面板显示电池",

        // -- Sound tab -----------------------------------------------------
        "Output Device" => "输出设备",
        "Input Device" => "输入设备",
        "Volume" => "音量",
        "Mute" => "静音",
        "Output Volume" => "输出音量",
        "Input Volume" => "输入音量",
        "Sound Effects" => "音效",
        "Alert Sound" => "提示音",
        "Over-Amplification" => "超幅放大",
        "Test" => "测试",
        "No output devices found" => "未找到输出设备",
        "No input devices found" => "未找到输入设备",

        // -- System tab ----------------------------------------------------
        "About" => "关于",
        "Device Name" => "设备名称",
        "OS" => "操作系统",
        "Kernel" => "内核",
        "Architecture" => "架构",
        "Graphics" => "显卡",
        "Memory" => "内存",
        "Processor" => "处理器",
        "Disk" => "磁盘",
        "Hostname" => "主机名",
        "Software Updates" => "软件更新",
        "Check for Updates" => "检查更新",
        "Last Checked" => "上次检查",
        "System Update" => "系统更新",
        "Downloading" => "正在下载",
        "Installing" => "正在安装",
        "Up to date" => "已是最新",
        "Restart Required" => "需要重启",
        "Update All" => "全部更新",
        "Language" => "语言",
        "Time & Date" => "时间和日期",
        "Time Zone" => "时区",
        "Date Format" => "日期格式",
        "Time Format" => "时间格式",
        "24-Hour" => "24 小时制",
        "Automatic Time" => "自动设置时间",

        // -- Fences (workspace grid) tab -----------------------------------
        "Fences" => "栅格布局",
        "Rows" => "行数",
        "Columns" => "列数",
        "Gap" => "间距",
        "Add Fence" => "添加栅格",
        "Remove Fence" => "移除栅格",
        "Fence Name" => "栅格名称",
        "Layout" => "布局",
        "Grid" => "网格",
        "Free" => "自由",

        // -- Template / format strings (callers apply format!()) -----------
        "fps_template" => "{fps} FPS",
        "resolution_template" => "{w}x{h}",
        "refresh_template" => "{rate} Hz",

        // -- Status bar (clock & battery) -----------------------------------
        "SUN" => "周日",
        "MON" => "周一",
        "TUE" => "周二",
        "WED" => "周三",
        "THU" => "周四",
        "FRI" => "周五",
        "SAT" => "周六",
        "JAN" => "1月",
        "FEB" => "2月",
        "MAR" => "3月",
        "APR" => "4月",
        "MAY" => "5月",
        "JUN" => "6月",
        "JUL" => "7月",
        "AUG" => "8月",
        "SEP" => "9月",
        "OCT" => "10月",
        "NOV" => "11月",
        "DEC" => "12月",
        "CHG" => "充电",
        "BAT" => "电池",

        // -- Lock screen ----------------------------------------------------
        "Enter password" => "输入密码",
        "Press Enter to unlock" => "按回车键解锁",
        "Unlock" => "解锁",
        "Locked" => "已锁定",

        // -- World picker (worlds panel) ------------------------------------
        "Delete?" => "删除？",
        "Yes" => "是",
        "No" => "否",
        "Open world picker" => "打开世界切换器",

        // -- Placeholder (Launch / Edit / Dismiss) --------------------------
        "Edit" => "编辑",
        "Launch" => "启动",

        // -- Placeholder settings -------------------------------------------
        "Restore" => "还原",
        "Active handler" => "活动处理器",
        "Switching the handler changes which synthesizer runs at launch. \
         Each handler's preferences are preserved when switching." =>
            "切换处理器会改变启动时运行的合成器。 \
             每个处理器的偏好会在切换时保留。",
        "capture" => "捕获",
        "(disabled — won't be passed to the launched process)" =>
            "（已禁用——不会传递给启动的进程）",

        // -- Capture dialogs ------------------------------------------------
        "Save As…" => "另存为…",
        "Capture setup" => "捕获设置",
        "Capturing the selected windows." => "正在捕获选中的窗口。",
        "Drag to draw a region (moves with the world)." => "拖动以绘制区域（随世界移动）。",
        "Drag to draw a region (fixed on screen)." => "拖动以绘制区域（固定于屏幕）。",
        "Capturing the whole screen." => "正在捕获全屏。",
        "Save to the default folder, choose a location, or discard." =>
            "保存到默认文件夹、选择位置或放弃。",
        "Optimized encoding (smaller file, encodes after saving)" =>
            "优化编码（文件更小，保存后编码）",
        "Capture error" => "捕获错误",
        "OK" => "确定",
        "Optimizing recording…" => "正在优化录制…",
        "Still capturing?" => "仍在录制？",
        "Stop" => "停止",

        // -- Navigator (keyboard shortcuts) ---------------------------------
        "Open launcher" => "打开启动器",
        "Screen capture" => "屏幕截图",
        "Exit fullscreen" => "退出全屏",
        "Zone 1" => "区域 1",
        "Zone 2" => "区域 2",
        "Zone 3" => "区域 3",
        "Zone 4" => "区域 4",
        "Zone 5" => "区域 5",
        "Zone 6" => "区域 6",
        "Set zone 1" => "设置区域 1",
        "Set zone 2" => "设置区域 2",
        "Set zone 3" => "设置区域 3",
        "Set zone 4" => "设置区域 4",
        "Set zone 5" => "设置区域 5",
        "Set zone 6" => "设置区域 6",
        "Group (join)" => "编组（加入）",
        "Group" => "编组",
        "Zoom: fit" => "缩放：适应",
        "Zoom: focus" => "缩放：聚焦",
        "Fit to screen" => "适应屏幕",
        "Navigate right" => "向右导航",
        "Navigate left" => "向左导航",
        "Navigate up" => "向上导航",
        "Navigate down" => "向下导航",
        "Navigate right and zoom" => "向右导航并缩放",
        "Navigate left and zoom" => "向左导航并缩放",
        "Navigate up and zoom" => "向上导航并缩放",
        "Navigate down and zoom" => "向下导航并缩放",
        "Move / pan window (hold + drag)" => "移动/平移窗口（按住 + 拖动）",
        "Scale window (hold + drag)" => "缩放窗口（按住 + 拖动）",
        "Select box (hold + drag)" => "选择框（按住 + 拖动）",
        "Select box, add (hold + drag)" => "选择框，添加（按住 + 拖动）",
        "Hand tool" => "手型工具",

        // -- Overlay keyboard shortcuts -------------------------------------
        "Switch to VT 1" => "切换到 VT 1",
        "Switch to VT 2" => "切换到 VT 2",
        "Switch to VT 3" => "切换到 VT 3",
        "Switch to VT 4" => "切换到 VT 4",
        "Switch to VT 5" => "切换到 VT 5",
        "Switch to VT 6" => "切换到 VT 6",
        "Cancel picker" => "取消选择器",
        "Volume up" => "音量增加",
        "Volume down" => "音量减小",
        "Play / Pause" => "播放 / 暂停",
        "Pause media" => "暂停媒体",
        "Stop media" => "停止媒体",
        "Next track" => "下一曲",
        "Previous track" => "上一曲",

        // -- Fallback ------------------------------------------------------
        _ => key,
    }
}
