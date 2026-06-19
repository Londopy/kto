// KTO GUI - a tabbed Win32 control panel for the kto.exe command-line tool.
//
// Tabs: Run (guided fields + a clickable network picker), Command (type raw kto
// flags, Zenmap-style), Settings (dark mode, close-to-tray, run-at-startup), and
// Wiki (how-to). A shared log at the bottom shows live output from whatever's
// running. Settings persist in HKCU\Software\KTO.
//
// Windows-only, plain Win32 - no external libraries.

#ifndef UNICODE
#define UNICODE
#endif
#include <windows.h>
#include <commctrl.h>
#include <shellapi.h>
#include <dwmapi.h>
#include <string>
#include <vector>
#include "resource.h"

#ifndef DWMWA_USE_IMMERSIVE_DARK_MODE
#define DWMWA_USE_IMMERSIVE_DARK_MODE 20
#endif

// Tabs.
enum { TAB_RUN = 0, TAB_CMD = 1, TAB_SETTINGS = 2, TAB_WIKI = 3 };

// Control ids.
enum {
    ID_TABS = 900,
    ID_IFACE = 1001, ID_TARGET, ID_MODE, ID_SIM, ID_START, ID_SCAN, ID_NETLIST,
    ID_CMD, ID_CMDRUN,
    ID_DARK, ID_TRAY, ID_STARTUP,
    ID_WIKI, ID_LOG, ID_STOP, ID_CLEAR,
    IDM_TRAY_RESTORE = 1500, IDM_TRAY_QUIT,
};

#define WM_APP_LOG  (WM_APP + 1)   // lParam = wchar_t* (UI thread frees it)
#define WM_APP_DONE (WM_APP + 2)
#define WM_APP_TRAY (WM_APP + 3)

static const wchar_t* RUN_KEY  = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
static const wchar_t* RUN_NAME = L"KTO";
static const wchar_t* CFG_KEY  = L"Software\\KTO";

// ---- globals --------------------------------------------------------------
static HWND  g_hwnd = nullptr, g_tabs = nullptr, g_log = nullptr, g_status = nullptr;
static HWND  g_iface = nullptr, g_target = nullptr, g_mode = nullptr, g_sim = nullptr;
static HWND  g_netlist = nullptr, g_cmd = nullptr;
static HWND  g_dark_chk = nullptr, g_tray_chk = nullptr, g_startup_chk = nullptr;
static std::vector<std::pair<HWND, int>> g_pages;   // (control, tab)
static std::vector<std::wstring> g_net_ssids;

static HANDLE g_childIn = nullptr, g_childOut = nullptr, g_proc = nullptr, g_reader = nullptr;
static bool   g_running = false;

static bool   g_dark = false, g_close_to_tray = false, g_force_quit = false, g_tray_shown = false;
static HBRUSH g_brBg = nullptr, g_brCtl = nullptr;
static const COLORREF DARK_BG = RGB(32, 32, 32), DARK_CTL = RGB(45, 45, 48), DARK_TX = RGB(232, 232, 232);

// ---- small helpers --------------------------------------------------------
static std::wstring self_path() {
    wchar_t buf[MAX_PATH];
    DWORD n = GetModuleFileNameW(nullptr, buf, MAX_PATH);
    return std::wstring(buf, n);
}
static std::wstring self_dir() {
    std::wstring p = self_path();
    size_t s = p.find_last_of(L"\\/");
    return s == std::wstring::npos ? L"." : p.substr(0, s);
}
static std::wstring kto_path() {
    std::wstring c = self_dir() + L"\\kto.exe";
    return GetFileAttributesW(c.c_str()) != INVALID_FILE_ATTRIBUTES ? c : L"kto.exe";
}
static std::wstring edit_text(HWND e) {
    int len = GetWindowTextLengthW(e);
    std::wstring s(len, L'\0');
    if (len) GetWindowTextW(e, &s[0], len + 1);
    return s;
}
static void log_append(const wchar_t* text) {
    int len = GetWindowTextLengthW(g_log);
    SendMessageW(g_log, EM_SETSEL, (WPARAM)len, (LPARAM)len);
    SendMessageW(g_log, EM_REPLACESEL, FALSE, (LPARAM)text);
}
static void log_line(const std::wstring& s) { log_append((s + L"\r\n").c_str()); }
static void set_status(const std::wstring& s) { SetWindowTextW(g_status, s.c_str()); }

// ---- settings (registry) --------------------------------------------------
static DWORD reg_dword(const wchar_t* name, DWORD def) {
    DWORD val = def, sz = sizeof(val);
    if (RegGetValueW(HKEY_CURRENT_USER, CFG_KEY, name, RRF_RT_REG_DWORD, nullptr, &val, &sz) != ERROR_SUCCESS)
        return def;
    return val;
}
static void reg_set_dword(const wchar_t* name, DWORD val) {
    RegSetKeyValueW(HKEY_CURRENT_USER, CFG_KEY, name, REG_DWORD, &val, sizeof(val));
}
static bool startup_enabled() {
    wchar_t buf[1024]; DWORD sz = sizeof(buf);
    return RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, RRF_RT_REG_SZ, nullptr, buf, &sz) == ERROR_SUCCESS;
}
static void set_startup(bool on) {
    if (on) {
        std::wstring cmd = L"\"" + self_path() + L"\"";
        RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, REG_SZ,
                        cmd.c_str(), (DWORD)((cmd.size() + 1) * sizeof(wchar_t)));
    } else {
        RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME);
    }
}

// ---- theme ----------------------------------------------------------------
static void apply_theme() {
    if (g_brBg) DeleteObject(g_brBg);
    if (g_brCtl) DeleteObject(g_brCtl);
    g_brBg = CreateSolidBrush(g_dark ? DARK_BG : GetSysColor(COLOR_BTNFACE));
    g_brCtl = CreateSolidBrush(g_dark ? DARK_CTL : GetSysColor(COLOR_WINDOW));
    BOOL dark = g_dark ? TRUE : FALSE;
    DwmSetWindowAttribute(g_hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark, sizeof(dark));
    InvalidateRect(g_hwnd, nullptr, TRUE);
    RedrawWindow(g_hwnd, nullptr, nullptr, RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW);
}

// ---- tray -----------------------------------------------------------------
static NOTIFYICONDATAW make_nid() {
    NOTIFYICONDATAW nid{};
    nid.cbSize = sizeof(nid);
    nid.hWnd = g_hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.hIcon = LoadIconW((HINSTANCE)GetWindowLongPtrW(g_hwnd, GWLP_HINSTANCE), MAKEINTRESOURCEW(IDI_APP));
    lstrcpyW(nid.szTip, L"KTO");
    return nid;
}
static void tray_show() {
    if (g_tray_shown) return;
    NOTIFYICONDATAW nid = make_nid();
    Shell_NotifyIconW(NIM_ADD, &nid);
    g_tray_shown = true;
}
static void tray_hide() {
    if (!g_tray_shown) return;
    NOTIFYICONDATAW nid = make_nid();
    Shell_NotifyIconW(NIM_DELETE, &nid);
    g_tray_shown = false;
}

// ---- child process --------------------------------------------------------
static DWORD WINAPI reader_thread(LPVOID param) {
    HANDLE out = (HANDLE)param;
    char buf[4096];
    DWORD n = 0;
    while (ReadFile(out, buf, sizeof(buf), &n, nullptr) && n > 0) {
        int wlen = MultiByteToWideChar(CP_UTF8, 0, buf, (int)n, nullptr, 0);
        if (wlen <= 0) continue;
        std::wstring w(wlen, L'\0');
        MultiByteToWideChar(CP_UTF8, 0, buf, (int)n, &w[0], wlen);
        std::wstring out2;
        out2.reserve(w.size() + 16);
        for (wchar_t c : w) {
            if (c == L'\n') out2 += L"\r\n";
            else if (c != L'\r') out2 += c;
        }
        wchar_t* heap = new wchar_t[out2.size() + 1];
        memcpy(heap, out2.c_str(), (out2.size() + 1) * sizeof(wchar_t));
        PostMessageW(g_hwnd, WM_APP_LOG, 0, (LPARAM)heap);
    }
    PostMessageW(g_hwnd, WM_APP_DONE, 0, 0);
    return 0;
}

static void set_running(bool running) {
    g_running = running;
    EnableWindow(GetDlgItem(g_hwnd, ID_START), !running);
    EnableWindow(GetDlgItem(g_hwnd, ID_CMDRUN), !running);
    EnableWindow(GetDlgItem(g_hwnd, ID_SCAN), !running);
    EnableWindow(GetDlgItem(g_hwnd, ID_STOP), running);
    set_status(running ? L"Running..." : L"Idle");
}

// Launch kto.exe with the given argument string, streaming output to the log.
static void start_process(const std::wstring& args) {
    if (g_running) return;

    SECURITY_ATTRIBUTES sa{};
    sa.nLength = sizeof(sa);
    sa.bInheritHandle = TRUE;
    HANDLE outRead = nullptr, outWrite = nullptr, inRead = nullptr, inWrite = nullptr;
    if (!CreatePipe(&outRead, &outWrite, &sa, 0) || !CreatePipe(&inRead, &inWrite, &sa, 0)) {
        log_line(L"[gui] failed to create pipes");
        return;
    }
    SetHandleInformation(outRead, HANDLE_FLAG_INHERIT, 0);
    SetHandleInformation(inWrite, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOW si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdOutput = outWrite;
    si.hStdError = outWrite;
    si.hStdInput = inRead;

    std::wstring cmd = L"\"" + kto_path() + L"\" " + args;
    std::vector<wchar_t> mut(cmd.begin(), cmd.end());
    mut.push_back(L'\0');

    PROCESS_INFORMATION pi{};
    BOOL ok = CreateProcessW(nullptr, mut.data(), nullptr, nullptr, TRUE,
                             CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi);
    CloseHandle(outWrite);
    CloseHandle(inRead);
    if (!ok) {
        CloseHandle(outRead);
        CloseHandle(inWrite);
        log_line(L"[gui] could not start kto.exe - is it next to this app or on PATH?");
        return;
    }
    CloseHandle(pi.hThread);
    g_proc = pi.hProcess;
    g_childOut = outRead;
    g_childIn = inWrite;
    g_reader = CreateThread(nullptr, 0, reader_thread, outRead, 0, nullptr);
    log_line(L"[gui] > kto " + args);
    set_running(true);
}

static void start_from_fields() {
    std::wstring target = edit_text(g_target);
    if (target.empty()) {
        MessageBoxW(g_hwnd, L"Enter a target SSID first (or pick one on the Run tab).", L"KTO", MB_ICONINFORMATION);
        return;
    }
    std::wstring iface = edit_text(g_iface);
    bool sim = SendMessageW(g_sim, BM_GETCHECK, 0, 0) == BST_CHECKED;
    int mode = (int)SendMessageW(g_mode, CB_GETCURSEL, 0, 0);
    std::wstring a = L"--no-tui --no-color --no-update-check";
    if (sim) a += L" --simulate";
    a += L" -i \"" + iface + L"\" -t \"" + target + L"\"";
    if (mode == 1) a += L" --aggressive";
    else if (mode == 2) a += L" --scan-only";
    start_process(a);
}

static void stop_process() {
    if (!g_running) return;
    const char nl = '\n';
    DWORD written = 0;
    if (g_childIn) WriteFile(g_childIn, &nl, 1, &written, nullptr);
}
static void cleanup_after_exit() {
    if (g_reader) { WaitForSingleObject(g_reader, 2000); CloseHandle(g_reader); g_reader = nullptr; }
    if (g_childOut) { CloseHandle(g_childOut); g_childOut = nullptr; }
    if (g_childIn) { CloseHandle(g_childIn); g_childIn = nullptr; }
    if (g_proc) { CloseHandle(g_proc); g_proc = nullptr; }
    set_running(false);
}

// Run a quick command to completion and return its stdout (UTF-8 -> wide).
static std::wstring run_capture(const std::wstring& args) {
    SECURITY_ATTRIBUTES sa{};
    sa.nLength = sizeof(sa);
    sa.bInheritHandle = TRUE;
    HANDLE rd = nullptr, wr = nullptr;
    if (!CreatePipe(&rd, &wr, &sa, 0)) return L"";
    SetHandleInformation(rd, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOW si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdOutput = wr;
    si.hStdError = wr;

    std::wstring cmd = L"\"" + kto_path() + L"\" " + args;
    std::vector<wchar_t> mut(cmd.begin(), cmd.end());
    mut.push_back(L'\0');

    PROCESS_INFORMATION pi{};
    if (!CreateProcessW(nullptr, mut.data(), nullptr, nullptr, TRUE, CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi)) {
        CloseHandle(rd); CloseHandle(wr);
        return L"";
    }
    CloseHandle(wr);
    std::string acc;
    char buf[4096];
    DWORD n = 0;
    while (ReadFile(rd, buf, sizeof(buf), &n, nullptr) && n > 0) acc.append(buf, n);
    WaitForSingleObject(pi.hProcess, 5000);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    CloseHandle(rd);

    int wlen = MultiByteToWideChar(CP_UTF8, 0, acc.data(), (int)acc.size(), nullptr, 0);
    std::wstring w(wlen, L'\0');
    if (wlen) MultiByteToWideChar(CP_UTF8, 0, acc.data(), (int)acc.size(), &w[0], wlen);
    return w;
}

static void scan_networks() {
    SendMessageW(g_netlist, LB_RESETCONTENT, 0, 0);
    g_net_ssids.clear();
    std::wstring out = run_capture(L"--list-networks --simulate");
    size_t start = 0;
    while (start < out.size()) {
        size_t nl = out.find(L'\n', start);
        std::wstring line = out.substr(start, (nl == std::wstring::npos ? out.size() : nl) - start);
        start = (nl == std::wstring::npos) ? out.size() : nl + 1;
        while (!line.empty() && (line.back() == L'\r' || line.back() == L' ')) line.pop_back();
        if (line.empty()) continue;
        // SSID \t BSSID \t CH \t RSSI
        size_t t1 = line.find(L'\t');
        std::wstring ssid = (t1 == std::wstring::npos) ? line : line.substr(0, t1);
        std::wstring rest = (t1 == std::wstring::npos) ? L"" : line.substr(t1 + 1);
        for (auto& c : rest) if (c == L'\t') c = L' ';
        g_net_ssids.push_back(ssid);
        std::wstring disp = ssid + L"   (" + rest + L")";
        SendMessageW(g_netlist, LB_ADDSTRING, 0, (LPARAM)disp.c_str());
    }
    set_status(L"Found " + std::to_wstring(g_net_ssids.size()) + L" networks");
}

// ---- control creation -----------------------------------------------------
static void apply_font(HWND c) { SendMessageW(c, WM_SETFONT, (WPARAM)GetStockObject(DEFAULT_GUI_FONT), TRUE); }

static HWND mk(const wchar_t* cls, const wchar_t* text, DWORD style,
               int x, int y, int w, int h, int id) {
    HWND c = CreateWindowExW(0, cls, text, WS_CHILD | style, x, y, w, h,
                             g_hwnd, (HMENU)(INT_PTR)id,
                             (HINSTANCE)GetWindowLongPtrW(g_hwnd, GWLP_HINSTANCE), nullptr);
    apply_font(c);
    return c;
}
// Tab-scoped control: created hidden, shown only on its tab.
static HWND page(int tab, const wchar_t* cls, const wchar_t* text, DWORD style,
                 int x, int y, int w, int h, int id) {
    HWND c = mk(cls, text, style, x, y, w, h, id);
    g_pages.push_back({c, tab});
    return c;
}

static void show_tab(int t) {
    for (auto& pr : g_pages)
        ShowWindow(pr.first, pr.second == t ? SW_SHOW : SW_HIDE);
}

static const wchar_t* WIKI_TEXT =
    L"KTO GUI - quick guide\r\n"
    L"=====================\r\n\r\n"
    L"RUN TAB\r\n"
    L"  - Interface: your monitor-mode adapter (e.g. wlan0mon). Only matters for\r\n"
    L"    the real radio path, which is stubbed in this build.\r\n"
    L"  - Target SSID: the network name to act on. Type it, or hit Scan and\r\n"
    L"    double-click one from the list.\r\n"
    L"  - Mode: Standard, Aggressive, or Scan only.\r\n"
    L"  - Simulate: keep this ON. The real attack path is not built in, so\r\n"
    L"    simulate mode is what shows activity safely.\r\n"
    L"  - Start runs it; Stop ends it cleanly; the log fills in live.\r\n\r\n"
    L"COMMAND TAB\r\n"
    L"  Type raw kto flags and hit Run - like the nmap/Zenmap command box.\r\n"
    L"  Example:  --simulate -t CorpNet --aggressive --no-tui --no-color\r\n"
    L"  Run 'kto --help' from a console to see every flag.\r\n\r\n"
    L"SETTINGS TAB\r\n"
    L"  - Dark mode: toggles the theme (saved between launches).\r\n"
    L"  - Minimize to tray on close: the X button hides to the tray instead of\r\n"
    L"    quitting. Right-click the tray icon for Restore / Quit.\r\n"
    L"  - Run at Windows startup: launches the GUI when you log in.\r\n\r\n"
    L"NOTES\r\n"
    L"  Authorized testing only. Deauthenticating networks you do not own, or\r\n"
    L"  lack written permission to test, is illegal in most places.\r\n";

static void create_controls() {
    // Tabs.
    g_tabs = mk(WC_TABCONTROLW, L"", WS_VISIBLE, 8, 8, 688, 26, ID_TABS);
    const wchar_t* names[] = {L"Run", L"Command", L"Settings", L"Wiki"};
    for (int i = 0; i < 4; i++) {
        TCITEMW it{};
        it.mask = TCIF_TEXT;
        it.pszText = (LPWSTR)names[i];
        SendMessageW(g_tabs, TCM_INSERTITEMW, i, (LPARAM)&it);
    }

    // --- Run tab ---
    page(TAB_RUN, L"STATIC", L"Interface:", SS_LEFT, 8, 50, 76, 20, 0);
    g_iface = page(TAB_RUN, L"EDIT", L"wlan0mon", WS_BORDER | ES_AUTOHSCROLL, 88, 48, 180, 24, ID_IFACE);
    page(TAB_RUN, L"STATIC", L"Target SSID:", SS_LEFT, 8, 82, 76, 20, 0);
    g_target = page(TAB_RUN, L"EDIT", L"", WS_BORDER | ES_AUTOHSCROLL, 88, 80, 180, 24, ID_TARGET);
    page(TAB_RUN, L"STATIC", L"Mode:", SS_LEFT, 8, 114, 76, 20, 0);
    g_mode = page(TAB_RUN, L"COMBOBOX", L"", CBS_DROPDOWNLIST | WS_VSCROLL, 88, 112, 180, 200, ID_MODE);
    SendMessageW(g_mode, CB_ADDSTRING, 0, (LPARAM)L"Standard");
    SendMessageW(g_mode, CB_ADDSTRING, 0, (LPARAM)L"Aggressive");
    SendMessageW(g_mode, CB_ADDSTRING, 0, (LPARAM)L"Scan only");
    SendMessageW(g_mode, CB_SETCURSEL, 0, 0);
    g_sim = page(TAB_RUN, L"BUTTON", L"Simulate (safe - no radio)", BS_AUTOCHECKBOX, 8, 144, 240, 22, ID_SIM);
    SendMessageW(g_sim, BM_SETCHECK, BST_CHECKED, 0);
    page(TAB_RUN, L"BUTTON", L"Start", BS_DEFPUSHBUTTON, 8, 172, 86, 30, ID_START);

    page(TAB_RUN, L"STATIC", L"Nearby networks (double-click to use):", SS_LEFT, 300, 30, 388, 18, 0);
    page(TAB_RUN, L"BUTTON", L"Scan", 0, 614, 46, 74, 26, ID_SCAN);
    g_netlist = page(TAB_RUN, L"LISTBOX", L"", WS_BORDER | WS_VSCROLL | LBS_NOTIFY, 300, 78, 388, 140, ID_NETLIST);

    // --- Command tab ---
    page(TAB_CMD, L"STATIC", L"kto", SS_LEFT, 8, 52, 28, 20, 0);
    g_cmd = page(TAB_CMD, L"EDIT", L"--simulate -t CorpNet --no-tui --no-color", WS_BORDER | ES_AUTOHSCROLL, 40, 50, 560, 24, ID_CMD);
    page(TAB_CMD, L"BUTTON", L"Run", BS_PUSHBUTTON, 610, 48, 80, 28, ID_CMDRUN);
    page(TAB_CMD, L"STATIC", L"Type kto flags then Run. See the Wiki tab, or run 'kto --help' in a console.", SS_LEFT, 8, 84, 680, 18, 0);

    // --- Settings tab ---
    g_dark_chk = page(TAB_SETTINGS, L"BUTTON", L"Dark mode", BS_AUTOCHECKBOX, 8, 52, 300, 22, ID_DARK);
    g_tray_chk = page(TAB_SETTINGS, L"BUTTON", L"Minimize to tray on close (instead of quitting)", BS_AUTOCHECKBOX, 8, 82, 460, 22, ID_TRAY);
    g_startup_chk = page(TAB_SETTINGS, L"BUTTON", L"Run KTO at Windows startup", BS_AUTOCHECKBOX, 8, 112, 360, 22, ID_STARTUP);
    page(TAB_SETTINGS, L"STATIC", L"Settings are saved automatically.", SS_LEFT, 8, 150, 680, 18, 0);

    // --- Wiki tab ---
    page(TAB_WIKI, L"EDIT", WIKI_TEXT, WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL,
         8, 44, 688, 182, ID_WIKI);

    // --- always-visible bottom strip ---
    mk(L"BUTTON", L"Stop", WS_VISIBLE, 8, 236, 86, 28, ID_STOP);
    mk(L"BUTTON", L"Clear log", WS_VISIBLE, 100, 236, 86, 28, ID_CLEAR);
    g_status = mk(L"STATIC", L"Idle", WS_VISIBLE | SS_LEFT, 200, 242, 496, 18, 0);
    g_log = mk(L"EDIT", L"", WS_VISIBLE | WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL,
               8, 270, 688, 150, ID_LOG);

    EnableWindow(GetDlgItem(g_hwnd, ID_STOP), FALSE);
    show_tab(TAB_RUN);
}

// ---- window proc ----------------------------------------------------------
static void tray_menu() {
    HMENU m = CreatePopupMenu();
    AppendMenuW(m, MF_STRING, IDM_TRAY_RESTORE, L"Restore");
    AppendMenuW(m, MF_STRING, IDM_TRAY_QUIT, L"Quit");
    POINT pt; GetCursorPos(&pt);
    SetForegroundWindow(g_hwnd);
    TrackPopupMenu(m, TPM_RIGHTBUTTON, pt.x, pt.y, 0, g_hwnd, nullptr);
    PostMessageW(g_hwnd, WM_NULL, 0, 0);
    DestroyMenu(m);
}

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CREATE:
        g_hwnd = hwnd;
        create_controls();
        g_dark = reg_dword(L"Dark", 0) != 0;
        g_close_to_tray = reg_dword(L"CloseToTray", 0) != 0;
        SendMessageW(g_dark_chk, BM_SETCHECK, g_dark ? BST_CHECKED : BST_UNCHECKED, 0);
        SendMessageW(g_tray_chk, BM_SETCHECK, g_close_to_tray ? BST_CHECKED : BST_UNCHECKED, 0);
        SendMessageW(g_startup_chk, BM_SETCHECK, startup_enabled() ? BST_CHECKED : BST_UNCHECKED, 0);
        apply_theme();
        return 0;

    case WM_NOTIFY: {
        LPNMHDR nh = (LPNMHDR)lp;
        if (nh->idFrom == ID_TABS && nh->code == TCN_SELCHANGE) {
            show_tab((int)SendMessageW(g_tabs, TCM_GETCURSEL, 0, 0));
            return 0;
        }
        break;
    }

    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case ID_START:  start_from_fields(); return 0;
        case ID_CMDRUN: start_process(edit_text(g_cmd)); return 0;
        case ID_STOP:   stop_process(); return 0;
        case ID_SCAN:   scan_networks(); return 0;
        case ID_CLEAR:  SetWindowTextW(g_log, L""); return 0;
        case ID_NETLIST:
            if (HIWORD(wp) == LBN_DBLCLK) {
                int i = (int)SendMessageW(g_netlist, LB_GETCURSEL, 0, 0);
                if (i >= 0 && i < (int)g_net_ssids.size()) {
                    SetWindowTextW(g_target, g_net_ssids[i].c_str());
                    set_status(L"Target set to " + g_net_ssids[i]);
                }
            }
            return 0;
        case ID_DARK:
            g_dark = SendMessageW(g_dark_chk, BM_GETCHECK, 0, 0) == BST_CHECKED;
            reg_set_dword(L"Dark", g_dark ? 1 : 0);
            apply_theme();
            return 0;
        case ID_TRAY:
            g_close_to_tray = SendMessageW(g_tray_chk, BM_GETCHECK, 0, 0) == BST_CHECKED;
            reg_set_dword(L"CloseToTray", g_close_to_tray ? 1 : 0);
            return 0;
        case ID_STARTUP:
            set_startup(SendMessageW(g_startup_chk, BM_GETCHECK, 0, 0) == BST_CHECKED);
            return 0;
        case IDM_TRAY_RESTORE:
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            tray_hide();
            return 0;
        case IDM_TRAY_QUIT:
            g_force_quit = true;
            tray_hide();
            DestroyWindow(hwnd);
            return 0;
        }
        return 0;

    case WM_APP_LOG: {
        wchar_t* t = (wchar_t*)lp;
        log_append(t);
        delete[] t;
        return 0;
    }
    case WM_APP_DONE:
        log_line(L"[gui] kto exited");
        cleanup_after_exit();
        return 0;

    case WM_APP_TRAY:
        if (LOWORD(lp) == WM_LBUTTONDBLCLK) {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            tray_hide();
        } else if (LOWORD(lp) == WM_RBUTTONUP) {
            tray_menu();
        }
        return 0;

    case WM_CTLCOLORSTATIC:
    case WM_CTLCOLOREDIT:
    case WM_CTLCOLORLISTBOX:
        if (g_dark) {
            HDC dc = (HDC)wp;
            SetTextColor(dc, DARK_TX);
            SetBkColor(dc, (msg == WM_CTLCOLORSTATIC) ? DARK_BG : DARK_CTL);
            return (LRESULT)((msg == WM_CTLCOLORSTATIC) ? g_brBg : g_brCtl);
        }
        return DefWindowProcW(hwnd, msg, wp, lp);

    case WM_ERASEBKGND:
        if (g_dark) {
            RECT rc; GetClientRect(hwnd, &rc);
            FillRect((HDC)wp, &rc, g_brBg);
            return 1;
        }
        return DefWindowProcW(hwnd, msg, wp, lp);

    case WM_CLOSE:
        if (g_running) stop_process();
        if (g_close_to_tray && !g_force_quit) {
            tray_show();
            ShowWindow(hwnd, SW_HIDE);
        } else {
            tray_hide();
            DestroyWindow(hwnd);
        }
        return 0;

    case WM_DESTROY:
        tray_hide();
        PostQuitMessage(0);
        return 0;
    }

    return DefWindowProcW(hwnd, msg, wp, lp);
}

int WINAPI wWinMain(HINSTANCE hInst, HINSTANCE, PWSTR, int nCmdShow) {
    InitCommonControls();

    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.hIcon = LoadIconW(hInst, MAKEINTRESOURCEW(IDI_APP));
    wc.hIconSm = wc.hIcon;
    wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
    wc.lpszClassName = L"KtoGuiWindow";
    RegisterClassExW(&wc);

    DWORD style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    RECT r{0, 0, 712, 432};
    AdjustWindowRect(&r, style, FALSE);

    HWND hwnd = CreateWindowExW(0, wc.lpszClassName, L"KTO - Kick Them Out",
                                style, CW_USEDEFAULT, CW_USEDEFAULT,
                                r.right - r.left, r.bottom - r.top,
                                nullptr, nullptr, hInst, nullptr);
    if (!hwnd) return 1;

    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    MSG msg;
    while (GetMessageW(&msg, nullptr, 0, 0) > 0) {
        if (!IsDialogMessageW(hwnd, &msg)) {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    tray_hide();
    return (int)msg.wParam;
}
