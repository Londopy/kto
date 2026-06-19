// KTO GUI - a small Win32 control panel that drives the kto.exe command-line
// tool, streams its output into a log box, and can toggle "run at startup".
//
// It builds a kto command line from the fields, launches it with its stdout
// piped back here, and shows the lines live. "Stop" writes a newline to kto's
// stdin, which is exactly what the CLI watches for to shut down cleanly.
//
// Windows-only. No external libraries - just the Win32 API.

#ifndef UNICODE
#define UNICODE
#endif
#include <windows.h>
#include <string>
#include <vector>
#include "resource.h"

// Control ids.
enum {
    ID_IFACE = 1001,
    ID_TARGET,
    ID_MODE,
    ID_SIM,
    ID_START,
    ID_STOP,
    ID_STARTUP,
    ID_LOG,
};

// Custom messages from the reader thread -> UI thread.
#define WM_APP_LOG  (WM_APP + 1)   // lParam = wchar_t* (UI thread frees it)
#define WM_APP_DONE (WM_APP + 2)

static const wchar_t* RUN_KEY  = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
static const wchar_t* RUN_NAME = L"KTO";

// Handles we hold while a kto process is running.
static HWND  g_hwnd      = nullptr;
static HWND  g_log       = nullptr;
static HANDLE g_childIn  = nullptr;   // write end of kto's stdin
static HANDLE g_childOut = nullptr;   // read end of kto's stdout
static HANDLE g_proc     = nullptr;
static HANDLE g_reader   = nullptr;
static bool  g_running   = false;

// ---- helpers --------------------------------------------------------------

static std::wstring self_path() {
    wchar_t buf[MAX_PATH];
    DWORD n = GetModuleFileNameW(nullptr, buf, MAX_PATH);
    return std::wstring(buf, n);
}

static std::wstring self_dir() {
    std::wstring p = self_path();
    size_t slash = p.find_last_of(L"\\/");
    return slash == std::wstring::npos ? L"." : p.substr(0, slash);
}

// kto.exe sitting next to this exe, otherwise just "kto.exe" (rely on PATH).
static std::wstring kto_path() {
    std::wstring candidate = self_dir() + L"\\kto.exe";
    if (GetFileAttributesW(candidate.c_str()) != INVALID_FILE_ATTRIBUTES)
        return candidate;
    return L"kto.exe";
}

static std::wstring edit_text(HWND edit) {
    int len = GetWindowTextLengthW(edit);
    std::wstring s(len, L'\0');
    if (len) GetWindowTextW(edit, &s[0], len + 1);
    return s;
}

static void append_log(const wchar_t* text) {
    int len = GetWindowTextLengthW(g_log);
    SendMessageW(g_log, EM_SETSEL, (WPARAM)len, (LPARAM)len);
    SendMessageW(g_log, EM_REPLACESEL, FALSE, (LPARAM)text);
}

static void append_line(const std::wstring& s) {
    std::wstring line = s + L"\r\n";
    append_log(line.c_str());
}

// ---- startup (HKCU Run key) ----------------------------------------------

static bool startup_enabled() {
    wchar_t buf[1024];
    DWORD sz = sizeof(buf);
    LONG r = RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME,
                          RRF_RT_REG_SZ, nullptr, buf, &sz);
    return r == ERROR_SUCCESS;
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
        // EDIT controls want CRLF; kto emits bare LF.
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
    EnableWindow(GetDlgItem(g_hwnd, ID_START), running ? FALSE : TRUE);
    EnableWindow(GetDlgItem(g_hwnd, ID_STOP),  running ? TRUE  : FALSE);
}

static void start_kto() {
    if (g_running) return;

    std::wstring iface  = edit_text(GetDlgItem(g_hwnd, ID_IFACE));
    std::wstring target = edit_text(GetDlgItem(g_hwnd, ID_TARGET));
    if (target.empty()) {
        MessageBoxW(g_hwnd, L"Enter a target SSID first.", L"KTO", MB_ICONINFORMATION);
        return;
    }
    bool sim = SendMessageW(GetDlgItem(g_hwnd, ID_SIM), BM_GETCHECK, 0, 0) == BST_CHECKED;
    int mode = (int)SendMessageW(GetDlgItem(g_hwnd, ID_MODE), CB_GETCURSEL, 0, 0);

    std::wstring cmd = L"\"" + kto_path() + L"\" --no-tui --no-color --no-update-check";
    if (sim) cmd += L" --simulate";
    cmd += L" -i \"" + iface + L"\" -t \"" + target + L"\"";
    if (mode == 1) cmd += L" --aggressive";
    else if (mode == 2) cmd += L" --scan-only";

    SECURITY_ATTRIBUTES sa{};
    sa.nLength = sizeof(sa);
    sa.bInheritHandle = TRUE;

    HANDLE outRead = nullptr, outWrite = nullptr;
    HANDLE inRead = nullptr, inWrite = nullptr;
    if (!CreatePipe(&outRead, &outWrite, &sa, 0) ||
        !CreatePipe(&inRead, &inWrite, &sa, 0)) {
        append_line(L"[gui] failed to create pipes");
        return;
    }
    // The parent's ends must not be inherited by the child.
    SetHandleInformation(outRead, HANDLE_FLAG_INHERIT, 0);
    SetHandleInformation(inWrite, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOW si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdOutput = outWrite;
    si.hStdError  = outWrite;
    si.hStdInput  = inRead;

    PROCESS_INFORMATION pi{};
    std::vector<wchar_t> mutable_cmd(cmd.begin(), cmd.end());
    mutable_cmd.push_back(L'\0');

    BOOL ok = CreateProcessW(nullptr, mutable_cmd.data(), nullptr, nullptr,
                             TRUE, CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi);
    // Child owns these now; close our copies.
    CloseHandle(outWrite);
    CloseHandle(inRead);
    if (!ok) {
        CloseHandle(outRead);
        CloseHandle(inWrite);
        append_line(L"[gui] could not start kto.exe - is it next to this app or on PATH?");
        return;
    }

    CloseHandle(pi.hThread);
    g_proc     = pi.hProcess;
    g_childOut = outRead;
    g_childIn  = inWrite;
    g_reader   = CreateThread(nullptr, 0, reader_thread, outRead, 0, nullptr);

    append_line(L"[gui] started: " + cmd);
    set_running(true);
}

static void stop_kto() {
    if (!g_running) return;
    // kto's CLI stops when it reads a line on stdin.
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

// ---- window ---------------------------------------------------------------

static void apply_font(HWND ctl) {
    SendMessageW(ctl, WM_SETFONT, (WPARAM)GetStockObject(DEFAULT_GUI_FONT), TRUE);
}

static HWND make(const wchar_t* cls, const wchar_t* text, DWORD style,
                 int x, int y, int w, int h, HWND parent, int id) {
    HWND c = CreateWindowExW(0, cls, text, WS_CHILD | WS_VISIBLE | style,
                             x, y, w, h, parent, (HMENU)(INT_PTR)id,
                             (HINSTANCE)GetWindowLongPtrW(parent, GWLP_HINSTANCE), nullptr);
    apply_font(c);
    return c;
}

static void create_controls(HWND hwnd) {
    make(L"STATIC", L"Interface:", 0, 16, 18, 80, 22, hwnd, 0);
    make(L"EDIT", L"wlan0mon", WS_BORDER | ES_AUTOHSCROLL, 100, 16, 200, 24, hwnd, ID_IFACE);

    make(L"STATIC", L"Target SSID:", 0, 16, 52, 80, 22, hwnd, 0);
    make(L"EDIT", L"", WS_BORDER | ES_AUTOHSCROLL, 100, 50, 200, 24, hwnd, ID_TARGET);

    make(L"STATIC", L"Mode:", 0, 320, 18, 50, 22, hwnd, 0);
    HWND mode = make(L"COMBOBOX", L"", CBS_DROPDOWNLIST | WS_VSCROLL, 370, 14, 150, 200, hwnd, ID_MODE);
    SendMessageW(mode, CB_ADDSTRING, 0, (LPARAM)L"Standard");
    SendMessageW(mode, CB_ADDSTRING, 0, (LPARAM)L"Aggressive");
    SendMessageW(mode, CB_ADDSTRING, 0, (LPARAM)L"Scan only");
    SendMessageW(mode, CB_SETCURSEL, 0, 0);

    HWND sim = make(L"BUTTON", L"Simulate (safe - no radio)", BS_AUTOCHECKBOX, 320, 50, 220, 24, hwnd, ID_SIM);
    SendMessageW(sim, BM_SETCHECK, BST_CHECKED, 0);

    make(L"BUTTON", L"Start", BS_DEFPUSHBUTTON, 16, 88, 90, 30, hwnd, ID_START);
    make(L"BUTTON", L"Stop", 0, 116, 88, 90, 30, hwnd, ID_STOP);

    HWND startup = make(L"BUTTON", L"Run KTO at Windows startup", BS_AUTOCHECKBOX, 230, 92, 290, 24, hwnd, ID_STARTUP);
    SendMessageW(startup, BM_SETCHECK, startup_enabled() ? BST_CHECKED : BST_UNCHECKED, 0);

    g_log = make(L"EDIT", L"", WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL,
                 16, 130, 600, 410, hwnd, ID_LOG);

    EnableWindow(GetDlgItem(hwnd, ID_STOP), FALSE);
}

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CREATE:
        g_hwnd = hwnd;
        create_controls(hwnd);
        return 0;

    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case ID_START: start_kto(); return 0;
        case ID_STOP:  stop_kto();  return 0;
        case ID_STARTUP:
            set_startup(SendMessageW(GetDlgItem(hwnd, ID_STARTUP), BM_GETCHECK, 0, 0) == BST_CHECKED);
            return 0;
        }
        return 0;

    case WM_APP_LOG: {
        wchar_t* text = (wchar_t*)lp;
        append_log(text);
        delete[] text;
        return 0;
    }

    case WM_APP_DONE:
        append_line(L"[gui] kto exited");
        cleanup_after_exit();
        return 0;

    case WM_CLOSE:
        if (g_running) stop_kto();
        DestroyWindow(hwnd);
        return 0;

    case WM_DESTROY:
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

    // Fixed-size window (no resize) keeps the layout simple.
    DWORD style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    RECT r{0, 0, 640, 580};
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
    return (int)msg.wParam;
}
